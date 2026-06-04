use std::collections::BTreeMap;
use serde::Serialize;
use worker::*;
use wilayah::{haversine_km, location_from_village, Location, Village};

#[derive(serde::Deserialize, Debug, Clone, Serialize)]
struct VillageRow {
    kode: String,
    nama: String,
    kecamatan: String,
    kota: String,
    provinsi: String,
    lat: f64,
    lon: f64,
}

impl From<&VillageRow> for Village {
    fn from(r: &VillageRow) -> Self {
        Village {
            code: r.kode.clone(),
            name: r.nama.clone(),
            district: r.kecamatan.clone(),
            city: r.kota.clone(),
            province: r.provinsi.clone(),
            lat: r.lat,
            lon: r.lon,
            dist_km: None,
        }
    }
}

#[derive(Serialize)]
struct IndexResponse {
    name: String,
    version: String,
    village_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nearest: Option<Vec<Village>>,
}

#[derive(Serialize)]
struct NearestResponse {
    results: Vec<Village>,
}

#[derive(Serialize)]
struct SearchResponse {
    results: Vec<VillageRow>,
}

#[derive(Serialize)]
struct CodeResponse {
    result: Option<VillageRow>,
}

#[derive(Serialize)]
struct CodePrefixResponse {
    results: Vec<VillageRow>,
    total: i64,
    has_more: bool,
}

#[derive(Serialize)]
struct LocateResponse {
    result: Option<Location>,
}

#[derive(Serialize)]
struct UpdateResponse {
    upserted: usize,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(serde::Deserialize)]
struct UpdatePayload {
    villages: Vec<VillageRow>,
}

#[derive(serde::Deserialize)]
struct MetaUpdatePayload {
    meta: BTreeMap<String, String>,
}

fn with_cors_and_cache(response: Result<Response>, max_age: u32) -> Result<Response> {
    response.map(|r| {
        let h = Headers::new();
        h.set("Access-Control-Allow-Origin", "*").unwrap();
        h.set("Access-Control-Allow-Methods", "GET, PUT, OPTIONS").unwrap();
        h.set("Access-Control-Allow-Headers", "*").unwrap();
        h.set("Cache-Control", &format!("public, max-age={}", max_age)).unwrap();
        r.with_headers(h)
    })
}

fn with_cors(response: Result<Response>) -> Result<Response> {
    response.map(|r| {
        let h = Headers::new();
        h.set("Access-Control-Allow-Origin", "*").unwrap();
        h.set("Access-Control-Allow-Methods", "GET, PUT, OPTIONS").unwrap();
        h.set("Access-Control-Allow-Headers", "*").unwrap();
        r.with_headers(h)
    })
}

fn error_response(msg: &str, status: u16) -> Result<Response> {
    let body = serde_json::to_string(&ErrorResponse {
        error: msg.to_string(),
    })
    .map_err(|e| Error::from(format!("serialize error: {e}")))?;
    with_cors(Response::error(&body, status))
}

fn check_auth(req: &Request, env: &Env) -> Result<()> {
    let token = env.secret("ADMIN_TOKEN")?.to_string();
    let auth = req.headers().get("Authorization")?.unwrap_or_default();
    if auth == format!("Bearer {token}") {
        Ok(())
    } else {
        Err(Error::from("Unauthorized"))
    }
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

fn parse_f64_param(url: &Url, key: &str) -> Option<f64> {
    query_param(url, key).and_then(|v| v.parse().ok())
}

fn parse_usize_param(url: &Url, key: &str, default: usize) -> usize {
    query_param(url, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

const INDEX_HTML: &str = include_str!("../examples/web-client/index.html");

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/", |req, ctx| async move {
            let is_html = req.headers().get("Accept")
                .ok().flatten()
                .map(|v| v.contains("text/html"))
                .unwrap_or(false);
            if is_html {
                let mut resp = Response::ok(INDEX_HTML)?;
                resp.headers_mut().set("Content-Type", "text/html; charset=utf-8")?;
                return with_cors(Ok(resp));
            }

            let d1 = ctx.env.d1("DB")?;
            let count: i64 = d1
                .prepare("SELECT COUNT(*) as cnt FROM locations")
                .first::<i64>(Some("cnt"))
                .await?
                .unwrap_or(0);

            let url = req.url()?;
            let mut source: Option<String> = None;
            let mut lat: Option<f64> = None;
            let mut lon: Option<f64> = None;

            // 1. Try query params first (GPS from device)
            if let (Some(lat_val), Some(lon_val)) = (parse_f64_param(&url, "lat"), parse_f64_param(&url, "lon")) {
                if (-90.0..=90.0).contains(&lat_val) && (-180.0..=180.0).contains(&lon_val) {
                    lat = Some(lat_val);
                    lon = Some(lon_val);
                    source = Some("gps".to_string());
                }
            }

            // 2. Fall back to Cloudflare IP geolocation headers
            if lat.is_none() {
                let cf_lat_str = req.headers().get("CF-IPLatitude").ok().flatten();
                let cf_lon_str = req.headers().get("CF-IPLongitude").ok().flatten();
                if let (Some(lat_val), Some(lon_val)) = (
                    cf_lat_str.and_then(|s| s.parse().ok()),
                    cf_lon_str.and_then(|s| s.parse().ok())
                ) {
                    lat = Some(lat_val);
                    lon = Some(lon_val);
                    source = Some("ip".to_string());
                }
            }

            let mut nearest: Option<Vec<Village>> = None;

            if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
                let limit = 5;
                let deltas: [f64; 10] = [0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 15.0, 45.0, 180.0];
                for &delta in &deltas {
                    let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
                        FROM locations \
                        WHERE lat BETWEEN ?1 AND ?2 AND lon BETWEEN ?3 AND ?4 \
                        LIMIT 50";
                    let stmt = d1.prepare(sql);
                    let query = stmt.bind(&[
                        (lat_val - delta).into(),
                        (lat_val + delta).into(),
                        (lon_val - delta).into(),
                        (lon_val + delta).into(),
                    ])?;

                    let rows: Vec<VillageRow> = query.all().await?.results()?;
                    if rows.is_empty() {
                        continue;
                    }

                    let mut candidates: Vec<Village> = rows
                        .iter()
                        .map(|r| {
                            let mut v = Village::from(r);
                            v.dist_km = Some(haversine_km(lat_val, lon_val, r.lat, r.lon));
                            v
                        })
                        .collect();
                    candidates.sort_by(|a, b| {
                        a.dist_km
                            .unwrap()
                            .partial_cmp(&b.dist_km.unwrap())
                            .unwrap()
                    });
                    candidates.truncate(limit);
                    nearest = Some(candidates);
                    break;
                }
            }

            with_cors_and_cache(Response::from_json(&IndexResponse {
                name: "wilayah".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                village_count: count,
                source,
                lat,
                lon,
                nearest,
            }), 30)
        })
        .get_async("/nearest", |req, ctx| async move {
            let url = req.url()?;
            let lat = match parse_f64_param(&url, "lat") {
                Some(v) if (-90.0..=90.0).contains(&v) => v,
                _ => return error_response("Invalid or missing 'lat' parameter", 400),
            };
            let lon = match parse_f64_param(&url, "lon") {
                Some(v) if (-180.0..=180.0).contains(&v) => v,
                _ => return error_response("Invalid or missing 'lon' parameter", 400),
            };
            let limit = parse_usize_param(&url, "limit", 5).clamp(1, 20);

            let d1 = ctx.env.d1("DB")?;

            let deltas: [f64; 10] =
                [0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 15.0, 45.0, 180.0];
            for &delta in &deltas {
                let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
                    FROM locations \
                    WHERE lat BETWEEN ?1 AND ?2 AND lon BETWEEN ?3 AND ?4 \
                    LIMIT 200";
                let stmt = d1.prepare(sql);
                let query = stmt.bind(&[
                    (lat - delta).into(),
                    (lat + delta).into(),
                    (lon - delta).into(),
                    (lon + delta).into(),
                ])?;

                let rows: Vec<VillageRow> = query.all().await?.results()?;
                if rows.is_empty() {
                    continue;
                }

                let mut candidates: Vec<Village> = rows
                    .iter()
                    .map(|r| {
                        let mut v = Village::from(r);
                        v.dist_km = Some(haversine_km(lat, lon, r.lat, r.lon));
                        v
                    })
                    .collect();
                candidates.sort_by(|a, b| {
                    a.dist_km
                        .unwrap()
                        .partial_cmp(&b.dist_km.unwrap())
                        .unwrap()
                });
                candidates.truncate(limit);
                return with_cors_and_cache(Response::from_json(&NearestResponse {
                    results: candidates,
                }), 30);
            }
            with_cors_and_cache(Response::from_json(&NearestResponse { results: vec![] }), 30)
        })
        .get_async("/search", |req, ctx| async move {
            let url = req.url()?;
            let q = match query_param(&url, "q") {
                Some(v) if !v.is_empty() => v,
                _ => return error_response("Query parameter 'q' is required", 400),
            };
            let limit = parse_usize_param(&url, "limit", 10).clamp(1, 100);
            let pattern = format!("%{q}%");

            let d1 = ctx.env.d1("DB")?;
            let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
                FROM locations \
                WHERE nama LIKE ?1 OR kecamatan LIKE ?1 \
                OR kota LIKE ?1 OR provinsi LIKE ?1 \
                LIMIT ?2";
            let stmt = d1.prepare(sql);
            let query = stmt.bind(&[pattern.into(), (limit as f64).into()])?;
            let rows: Vec<VillageRow> = query.all().await?.results()?;
            with_cors_and_cache(Response::from_json(&SearchResponse { results: rows }), 60)
        })
        .get_async("/code", |req, ctx| async move {
            let url = req.url()?;
            let d1 = ctx.env.d1("DB")?;

            if let Some(q) = query_param(&url, "q") {
                let stmt = d1.prepare(
                    "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
                    FROM locations WHERE kode = ?1",
                );
                let query = stmt.bind(&[q.into()])?;
                let result: Option<VillageRow> = query.first(None).await?;
                return with_cors_and_cache(Response::from_json(&CodeResponse { result }), 3600);
            }

            if let Some(prefix) = query_param(&url, "prefix") {
                let limit = parse_usize_param(&url, "limit", 100).clamp(1, 1000);
                let offset = parse_usize_param(&url, "offset", 0);
                let pattern = format!("{prefix}%");

                let count_sql =
                    "SELECT COUNT(*) as cnt FROM locations WHERE kode LIKE ?1";
                let count_stmt = d1.prepare(count_sql);
                let count_query = count_stmt.bind(&[pattern.clone().into()])?;
                let total: i64 = count_query
                    .first::<i64>(Some("cnt"))
                    .await?
                    .unwrap_or(0);

                let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
                    FROM locations WHERE kode LIKE ?1 \
                    ORDER BY kode LIMIT ?2 OFFSET ?3";
                let stmt = d1.prepare(sql);
                let query = stmt.bind(&[
                    pattern.into(),
                    (limit as f64).into(),
                    (offset as f64).into(),
                ])?;
                let rows: Vec<VillageRow> = query.all().await?.results()?;
                let has_more = (offset + rows.len()) < total as usize;
                return with_cors_and_cache(Response::from_json(&CodePrefixResponse {
                    results: rows,
                    total,
                    has_more,
                }), 3600);
            }

            error_response(
                "Provide either 'q' (exact code) or 'prefix' (code prefix)",
                400,
            )
        })
        .get_async("/locate", |req, ctx| async move {
            let url = req.url()?;
            let lat = match parse_f64_param(&url, "lat") {
                Some(v) if (-90.0..=90.0).contains(&v) => v,
                _ => return error_response("Invalid or missing 'lat' parameter", 400),
            };
            let lon = match parse_f64_param(&url, "lon") {
                Some(v) if (-180.0..=180.0).contains(&v) => v,
                _ => return error_response("Invalid or missing 'lon' parameter", 400),
            };

            let d1 = ctx.env.d1("DB")?;

            let deltas: [f64; 10] =
                [0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 15.0, 45.0, 180.0];
            for &delta in &deltas {
                let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
                    FROM locations \
                    WHERE lat BETWEEN ?1 AND ?2 AND lon BETWEEN ?3 AND ?4 \
                    LIMIT 200";
                let stmt = d1.prepare(sql);
                let query = stmt.bind(&[
                    (lat - delta).into(),
                    (lat + delta).into(),
                    (lon - delta).into(),
                    (lon + delta).into(),
                ])?;

                let rows: Vec<VillageRow> = query.all().await?.results()?;
                if rows.is_empty() {
                    continue;
                }

                let mut candidates: Vec<(VillageRow, f64)> = rows
                    .into_iter()
                    .map(|r| {
                        let d = haversine_km(lat, lon, r.lat, r.lon);
                        (r, d)
                    })
                    .collect();
                candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                if let Some((village_row, dist_km)) = candidates.into_iter().next() {
                    let village = Village::from(&village_row);
                    if let Some(loc) = location_from_village(&village, dist_km) {
                        return with_cors_and_cache(Response::from_json(&LocateResponse {
                            result: Some(loc),
                        }), 30);
                    }
                }
            }
        with_cors_and_cache(Response::from_json(&LocateResponse { result: None }), 30)
    })
        .put_async("/update", |mut req, ctx| async move {
            if let Err(_) = check_auth(&req, &ctx.env) {
                return error_response("Unauthorized", 401);
            }
            let payload: UpdatePayload = match req.json().await {
                Ok(p) => p,
                Err(_) => return error_response("Invalid JSON body", 400),
            };
            if payload.villages.is_empty() {
                return error_response("No villages provided", 400);
            }
            if payload.villages.len() > 500 {
                return error_response("Max 500 villages per request", 400);
            }

            let d1 = ctx.env.d1("DB")?;
            let stmt = d1.prepare(
                "INSERT OR REPLACE INTO locations (kode, nama, kecamatan, kota, provinsi, lat, lon) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            );

            let rows: Vec<Vec<D1Type>> = payload.villages.iter().map(|v| {
                vec![
                    D1Type::Text(&v.kode),
                    D1Type::Text(&v.nama),
                    D1Type::Text(&v.kecamatan),
                    D1Type::Text(&v.kota),
                    D1Type::Text(&v.provinsi),
                    D1Type::Real(v.lat),
                    D1Type::Real(v.lon),
                ]
            }).collect();

            let row_refs: Vec<Vec<&D1Type>> = rows.iter().map(|r| r.iter().collect()).collect();
            let stmts = stmt.batch_bind(row_refs)?;
            let chunked: Vec<Vec<D1PreparedStatement>> = stmts.chunks(100).map(|c| c.to_vec()).collect();
            // Each batch() call counts as 1 subrequest regardless of statement count,
            // which fits comfortably within the free tier 50 subrequest limit.
            for chunk in chunked {
                d1.batch(chunk).await?;
            }

            with_cors(Response::from_json(&UpdateResponse { upserted: payload.villages.len() }))
        })
        .put_async("/update/meta", |mut req, ctx| async move {
            if let Err(_) = check_auth(&req, &ctx.env) {
                return error_response("Unauthorized", 401);
            }
            let payload: MetaUpdatePayload = match req.json().await {
                Ok(p) => p,
                Err(_) => return error_response("Invalid JSON body", 400),
            };
            if payload.meta.is_empty() {
                return error_response("No metadata provided", 400);
            }

            let d1 = ctx.env.d1("DB")?;
            let stmt = d1.prepare(
                "INSERT OR REPLACE INTO db_meta (key, value) VALUES (?1, ?2)",
            );

            let rows: Vec<Vec<D1Type>> = payload.meta.iter().map(|(k, v)| {
                vec![D1Type::Text(k), D1Type::Text(v)]
            }).collect();

            let row_refs: Vec<Vec<&D1Type>> = rows.iter().map(|r| r.iter().collect()).collect();
            let stmts = stmt.batch_bind(row_refs)?;
            let chunked: Vec<Vec<D1PreparedStatement>> = stmts.chunks(100).map(|c| c.to_vec()).collect();
            for chunk in chunked {
                d1.batch(chunk).await?;
            }

            with_cors(Response::from_json(&UpdateResponse { upserted: payload.meta.len() }))
        })
        .options_async("/*catchall", |_req, _ctx| async move {
            let h = Headers::new();
            h.set("Access-Control-Allow-Origin", "*").unwrap();
            h.set("Access-Control-Allow-Methods", "GET, PUT, OPTIONS").unwrap();
            h.set("Access-Control-Allow-Headers", "*").unwrap();
            Ok(Response::empty()?.with_headers(h))
        })
        .run(req, env)
        .await
}

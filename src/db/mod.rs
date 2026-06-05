use worker::*;
use crate::models::VillageRow;
use crate::utils::constants::{DELTAS, SQL_LIMIT};

pub async fn get_village_count(env: &Env) -> Result<i64> {
    let d1 = env.d1("DB")?;
    let count: i64 = d1
        .prepare("SELECT COUNT(*) as cnt FROM locations")
        .first::<i64>(Some("cnt"))
        .await?
        .unwrap_or(0);
    Ok(count)
}

pub async fn query_nearest_candidates(
    env: &Env,
    lat: f64,
    lon: f64,
) -> Result<Vec<VillageRow>> {
    let d1 = env.d1("DB")?;
    for &delta in &DELTAS {
        let sql = format!(
            "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
             FROM locations \
             WHERE lat BETWEEN ?1 AND ?2 AND lon BETWEEN ?3 AND ?4 \
             LIMIT {}",
            SQL_LIMIT
        );
        let stmt = d1.prepare(&sql);
        let query = stmt.bind(&[
            (lat - delta).into(),
            (lat + delta).into(),
            (lon - delta).into(),
            (lon + delta).into(),
        ])?;

        let rows: Vec<VillageRow> = query.all().await?.results()?;
        if !rows.is_empty() {
            return Ok(rows);
        }
    }
    Ok(vec![])
}

pub async fn search_villages(env: &Env, pattern: &str, limit: usize) -> Result<Vec<VillageRow>> {
    let d1 = env.d1("DB")?;
    let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
        FROM locations \
        WHERE nama LIKE ?1 OR kecamatan LIKE ?1 \
        OR kota LIKE ?1 OR provinsi LIKE ?1 \
        LIMIT ?2";
    let query = d1.prepare(sql).bind(&[pattern.into(), (limit as f64).into()])?;
    let rows: Vec<VillageRow> = query.all().await?.results()?;
    Ok(rows)
}

pub async fn get_village_by_code(env: &Env, code: &str) -> Result<Option<VillageRow>> {
    let d1 = env.d1("DB")?;
    let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
        FROM locations WHERE kode = ?1";
    let query = d1.prepare(sql).bind(&[code.into()])?;
    let result: Option<VillageRow> = query.first(None).await?;
    Ok(result)
}

pub async fn get_village_count_by_prefix(env: &Env, pattern: &str) -> Result<i64> {
    let d1 = env.d1("DB")?;
    let sql = "SELECT COUNT(*) as cnt FROM locations WHERE kode LIKE ?1";
    let count: i64 = d1
        .prepare(sql)
        .bind(&[pattern.into()])?
        .first::<i64>(Some("cnt"))
        .await?
        .unwrap_or(0);
    Ok(count)
}

pub async fn get_villages_by_prefix(
    env: &Env,
    pattern: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<VillageRow>> {
    let d1 = env.d1("DB")?;
    let sql = "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon \
        FROM locations WHERE kode LIKE ?1 \
        ORDER BY kode LIMIT ?2 OFFSET ?3";
    let query = d1.prepare(sql).bind(&[
        pattern.into(),
        (limit as f64).into(),
        (offset as f64).into(),
    ])?;
    let rows: Vec<VillageRow> = query.all().await?.results()?;
    Ok(rows)
}

pub async fn update_villages(env: &Env, villages: &[VillageRow]) -> Result<()> {
    let d1 = env.d1("DB")?;
    let stmt = d1.prepare(
        "INSERT OR REPLACE INTO locations (kode, nama, kecamatan, kota, provinsi, lat, lon) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    );

    let rows: Vec<Vec<D1Type>> = villages.iter().map(|v| {
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
    for chunk in chunked {
        d1.batch(chunk).await?;
    }
    Ok(())
}

pub async fn update_meta(
    env: &Env,
    meta: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    let d1 = env.d1("DB")?;
    let stmt = d1.prepare(
        "INSERT OR REPLACE INTO db_meta (key, value) VALUES (?1, ?2)",
    );

    let rows: Vec<Vec<D1Type>> = meta.iter().map(|(k, v)| {
        vec![D1Type::Text(k), D1Type::Text(v)]
    }).collect();

    let row_refs: Vec<Vec<&D1Type>> = rows.iter().map(|r| r.iter().collect()).collect();
    let stmts = stmt.batch_bind(row_refs)?;
    let chunked: Vec<Vec<D1PreparedStatement>> = stmts.chunks(100).map(|c| c.to_vec()).collect();
    for chunk in chunked {
        d1.batch(chunk).await?;
    }
    Ok(())
}

# wilayah Cloudflare Worker

A Cloudflare Worker that serves the wilayah Indonesian village lookup API, backed by Cloudflare D1 (managed SQLite).

## Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | API info, village count, and optionally nearest villages by location |
| `GET /nearest?lat=&lon=&limit=` | Find nearest villages by coordinates |
| `GET /search?q=&limit=` | Search villages by name (LIKE) |
| `GET /code?q=` | Exact code lookup |
| `GET /code?prefix=&limit=&offset=` | Code prefix lookup with pagination |
| `GET /locate?lat=&lon=` | Reverse-geocode to administrative hierarchy |
| `PUT /update` | Upsert village records (auth required) |
| `PUT /update/meta` | Upsert db_meta key-value pairs (auth required) |

All responses are JSON with CORS headers.

## Setup

### Prerequisites

- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [Node.js](https://nodejs.org/) (for wrangler)
- [Wrangler](https://developers.cloudflare.com/workers/wrangler/): `npm install -g wrangler`
- A Cloudflare account with D1 access

### 1. Add wasm32 target

```bash
rustup target add wasm32-unknown-unknown
```

### 2. Create a D1 database

```bash
wrangler d1 create wilayah-locations
```

Copy the `database_id` from the output and create your local `wrangler.toml` from the template:

```bash
cp wrangler.template.toml wrangler.toml
# Edit wrangler.toml to fill in your database_id
```

### 3. Import data

Download the pre-built `locations.db` from the latest release:

```bash
curl -L -o locations.db https://github.com/rafiyq/wilayah/releases/download/v0.5.0/locations.db
```

Apply the schema and import data into your local D1:

```bash
# Apply schema
wrangler d1 execute DB --local --file=schema.sql

# Clear existing data (for re-population)
wrangler d1 execute DB --local --command="DELETE FROM locations;"

# Export data to SQL
sqlite3 locations.db -cmd ".mode insert locations" \
  "SELECT id, kode, nama, kecamatan, kota, provinsi, lat, lon FROM locations WHERE lat != 0 OR lon != 0 ORDER BY id;" \
  > locations_data.sql

# Import in batches (D1 local has limits on large queries)
BATCH_SIZE=5000
CURRENT_BATCH=""
ROW_COUNT=0
BATCH_NUM=0

while IFS= read -r line; do
  CURRENT_BATCH="${CURRENT_BATCH}${line}"$'\n'
  ROW_COUNT=$((ROW_COUNT + 1))
  if [ "$ROW_COUNT" -ge "$BATCH_SIZE" ]; then
    BATCH_NUM=$((BATCH_NUM + 1))
    echo "Importing batch ${BATCH_NUM} (${ROW_COUNT} rows)..."
    echo "PRAGMA ignore_check_constraints = true;" > batch_tmp.sql
    echo "$CURRENT_BATCH" >> batch_tmp.sql
    wrangler d1 execute DB --local --file=batch_tmp.sql
    CURRENT_BATCH=""
    ROW_COUNT=0
  fi
done < locations_data.sql

# Import remaining rows
if [ "$ROW_COUNT" -gt 0 ]; then
  BATCH_NUM=$((BATCH_NUM + 1))
  echo "Importing final batch ${BATCH_NUM} (${ROW_COUNT} rows)..."
  echo "PRAGMA ignore_check_constraints = true;" > batch_tmp.sql
  echo "$CURRENT_BATCH" >> batch_tmp.sql
  wrangler d1 execute DB --local --file=batch_tmp.sql
fi

rm -f batch_tmp.sql locations_data.sql
```

### 4. Set up authentication

The `PUT /update` and `PUT /update/meta` endpoints require a Bearer token. Set the `ADMIN_TOKEN` secret via wrangler:

```bash
wrangler secret put ADMIN_TOKEN
```

Then pass the token in the `Authorization` header when calling the endpoints.

### 5. Updating data via API

After the initial import, you can incrementally update village records and metadata via the API:

```bash
# Upsert village records
curl -X PUT https://your-worker.workers.dev/update \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"villages":[{"kode":"33.12.24.2002","nama":"Sukodono","kecamatan":"Sukodono","kota":"Kab. Sragen","provinsi":"Jawa Tengah","lat":-7.43,"lon":111.02}]}'

# Upsert metadata
curl -X PUT https://your-worker.workers.dev/update/meta \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"meta":{"data_version":"2025-01","village_count":"83468"}}'
```

- `PUT /update` accepts up to 500 villages per request.
- `PUT /update/meta` accepts any number of key-value pairs.
- Both use `INSERT OR REPLACE` (upsert) semantics.

### 6. Run locally

```bash
npx wrangler dev
```

### 7. Deploy manually

```bash
npx wrangler deploy
```

## Device Location

The `GET /` endpoint supports location-aware responses. It tries sources in this order:

1. **`?lat=&lon=` query parameters** (e.g., from device GPS)
2. **Cloudflare IP geolocation headers** (`CF-IPLatitude`, `CF-IPLongitude`)
3. **No location** — returns API info + village count only

### Example with GPS

```javascript
navigator.geolocation.getCurrentPosition(async (position) => {
  const lat = position.coords.latitude;
  const lon = position.coords.longitude;
  const res = await fetch(`https://api.wilayah.workers.dev/?lat=${lat}&lon=${lon}`);
  const data = await res.json();
  console.log(data.nearest); // Array of nearby villages
});
```

### Example with no location

```bash
curl https://api.wilayah.workers.dev/
# Response: { "name": "wilayah", "version": "0.1.0", "village_count": 83468 }
```

### Web Client Example

An interactive demo is available in `examples/web-client/index.html`. Open it in a browser, allow location access, and it will show the nearest villages to your current GPS position.

## CI Deployment (GitHub Actions)

This project uses a tag-based release workflow. When you push a tag starting with `v`, GitHub Actions will:

1. **Populate D1:** Re-populate the D1 database with data from the latest release.
2. **Deploy Worker:** Build and deploy the Cloudflare Worker.

### Required GitHub secrets and variables

Go to **Settings → Secrets and variables → Actions** in your GitHub repository and set:

| Name | Type | Description |
|------|------|-------------|
| `CLOUDFLARE_API_TOKEN` | Secret | Wrangler API token (from Cloudflare dashboard) |
| `CLOUDFLARE_ACCOUNT_ID` | Secret | Your Cloudflare account ID |
| `D1_DATABASE_ID` | Variable | The D1 database ID from `wrangler d1 create` |

### How to release

```bash
# Create a new release tag
git tag v1.0.0
git push origin v1.0.0
```

This will trigger the workflow, which will first populate D1 with the latest data, then deploy the worker.

## Differences from the axum server example

| Feature | axum `serve.rs` | Cloudflare Worker |
|---------|-----------------|-------------------|
| Database | Embedded in-memory SQLite | Cloudflare D1 (remote) |
| Spatial index | RTree + Haversine UDF | Bounding box + Rust-side Haversine |
| Full-text search | FTS5 (BM25 ranked) | LIKE queries |
| Runtime | Tokio async runtime | Workers runtime (WASM) |
| Scaling | Single process | Edge network, global |

The Cloudflare Worker trades some query precision (no FTS5 ranking, no RTree index) for global edge deployment with zero infrastructure management. For most use cases, the bounding-box + Haversine approach gives equivalent results to the RTree approach for the `/nearest` and `/locate` endpoints.

CREATE TABLE IF NOT EXISTS locations (
    id INTEGER PRIMARY KEY,
    kode TEXT NOT NULL UNIQUE,
    nama TEXT NOT NULL,
    kecamatan TEXT NOT NULL,
    kota TEXT NOT NULL,
    provinsi TEXT NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_locations_lat_lon ON locations(lat, lon);
CREATE INDEX IF NOT EXISTS idx_locations_kode ON locations(kode);

CREATE TABLE IF NOT EXISTS db_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

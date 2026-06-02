#!/usr/bin/env bash
set -euo pipefail

DB_PATH="${1:-../../data/locations.db}"
DB_NAME="${2:-wilayah-locations}"

if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database not found at $DB_PATH"
    echo "Run 'cargo run --example build_db --features build-db' first"
    exit 1
fi

if ! command -v wrangler &>/dev/null; then
    echo "Error: wrangler not found. Install with: npm install -g wrangler"
    exit 1
fi

if ! command -v sqlite3 &>/dev/null; then
    echo "Error: sqlite3 CLI not found."
    exit 1
fi

echo "Creating D1 database (if not exists)..."
DB_ID=$(wrangler d1 list 2>/dev/null | grep "$DB_NAME" | awk '{print $1}' || true)

if [ -z "$DB_ID" ]; then
    echo "Creating new D1 database: $DB_NAME"
    CREATE_OUTPUT=$(wrangler d1 create "$DB_NAME" 2>&1)
    DB_ID=$(echo "$CREATE_OUTPUT" | grep -oP 'database_id = "\K[^"]+' || true)
    if [ -z "$DB_ID" ]; then
        echo "Error: Failed to create D1 database or extract database_id"
        echo "$CREATE_OUTPUT"
        exit 1
    fi
    echo "Created D1 database with ID: $DB_ID"
    echo ""
    echo "Update your wrangler.toml with:"
    echo "  database_id = \"$DB_ID\""
    echo ""
else
    echo "Using existing D1 database: $DB_NAME ($DB_ID)"
fi

SCHEMA_FILE="$(dirname "$0")/schema.sql"
echo "Applying schema..."
wrangler d1 execute "$DB_NAME" --file="$SCHEMA_FILE" --remote

echo "Exporting data from $DB_PATH..."
DATA_SQL=$(sqlite3 "$DB_PATH" -cmd ".mode insert locations" "SELECT kode, nama, kecamatan, kota, provinsi, lat, lon FROM locations WHERE lat != 0 OR lon != 0 ORDER BY id;")

if [ -z "$DATA_SQL" ]; then
    echo "Error: No data exported from $DB_PATH"
    exit 1
fi

ROW_COUNT=$(echo "$DATA_SQL" | grep -c "^INSERT" || true)
echo "Exported $ROW_COUNT rows"

echo "Importing data into D1 (this may take a few minutes)..."

# Write to temp file to avoid shell argument length limits
TEMP_FILE=$(mktemp)
echo "$DATA_SQL" > "$TEMP_FILE"

# Split into batches of 5000 INSERT statements to avoid D1 request size limits
BATCH_SIZE=5000
BATCH_NUM=0
CURRENT_BATCH=""
BATCH_COUNT=0

while IFS= read -r line; do
    CURRENT_BATCH="${CURRENT_BATCH}${line}"$'\n'
    BATCH_COUNT=$((BATCH_COUNT + 1))
    if [ "$BATCH_COUNT" -ge "$BATCH_SIZE" ]; then
        BATCH_NUM=$((BATCH_NUM + 1))
        echo "  Importing batch $BATCH_NUM ($BATCH_COUNT rows)..."
        echo "$CURRENT_BATCH" | wrangler d1 execute "$DB_NAME" --remote --file=-
        CURRENT_BATCH=""
        BATCH_COUNT=0
    fi
done < "$TEMP_FILE"

# Import remaining rows
if [ "$BATCH_COUNT" -gt 0 ]; then
    BATCH_NUM=$((BATCH_NUM + 1))
    echo "  Importing batch $BATCH_NUM ($BATCH_COUNT rows)..."
    echo "$CURRENT_BATCH" | wrangler d1 execute "$DB_NAME" --remote --file=-
fi

rm -f "$TEMP_FILE"

echo "Exporting db_meta from $DB_PATH..."
META_SQL=$(sqlite3 "$DB_PATH" -cmd ".mode insert db_meta" "SELECT key, value FROM db_meta;" 2>/dev/null || true)

if [ -n "$META_SQL" ]; then
    META_ROW_COUNT=$(echo "$META_SQL" | grep -c "^INSERT" || true)
    echo "Exported $META_ROW_COUNT meta rows"
    echo "Importing metadata into D1..."
    echo "$META_SQL" | wrangler d1 execute "$DB_NAME" --remote --file=-
else
    echo "No db_meta table found in source database, skipping metadata import"
fi

echo ""
echo "Import complete! Verify with:"
echo "  wrangler d1 execute $DB_NAME --remote --command='SELECT COUNT(*) FROM locations'"
echo "  wrangler d1 execute $DB_NAME --remote --command='SELECT * FROM db_meta'"

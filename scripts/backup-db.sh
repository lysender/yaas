#!/bin/sh

# env vars required
# DB_BACKUP_PATH=/path/to/db-backups
# YAAS_DB_PATH=/path/to/db
# BACKUP_S3_BUCKET=your-s3-bucket-name

CURRENT_DATE=$(date +"%Y-%m-%d-%H-%M-%S")
TARGET_DIR="$DB_BACKUP_PATH/yaas/$CURRENT_DATE"
BACKUP_FILE="yaas-db-$CURRENT_DATE.tar.gz"

echo "Creating backup for yaas database at $CURRENT_DATE"

# Create the backup dir
mkdir -p "$TARGET_DIR"

# Backup the database
tursodb --readonly "$YAAS_DB_PATH/yaas.db" ".dump" >"$TARGET_DIR/yaas.sql"

# Compress directory
cd "$DB_BACKUP_PATH/yaas"
tar czf "$BACKUP_FILE" "$CURRENT_DATE"

# Upload to S3
aws s3 cp "$BACKUP_FILE" "s3://$BACKUP_S3_BUCKET/db-backups/yaas/$BACKUP_FILE"

# Cleanup
rm -rf $TARGET_DIR
rm $BACKUP_FILE

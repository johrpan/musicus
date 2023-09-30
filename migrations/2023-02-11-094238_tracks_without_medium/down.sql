CREATE TABLE "old_tracks" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "medium" TEXT NOT NULL REFERENCES "mediums"("id") ON DELETE CASCADE,
    "index" INTEGER NOT NULL,
    "recording" TEXT NOT NULL REFERENCES "recordings"("id"),
    "work_parts" TEXT NOT NULL,
    "source_index" INTEGER NOT NULL,
    "path" TEXT NOT NULL,
    "last_used" BIGINT,
    "last_played" BIGINT
);

INSERT INTO "old_tracks" SELECT * FROM "tracks" WHERE "medium" IS NOT NULL;
DROP TABLE "tracks";
ALTER TABLE "old_tracks" RENAME TO "tracks";
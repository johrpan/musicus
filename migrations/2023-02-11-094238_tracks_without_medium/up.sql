CREATE TABLE "new_tracks" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "medium" TEXT REFERENCES "mediums"("id") ON DELETE CASCADE,
    "index" INTEGER NOT NULL,
    "recording" TEXT NOT NULL REFERENCES "recordings"("id"),
    "work_parts" TEXT NOT NULL,
    "source_index" INTEGER NOT NULL,
    "path" TEXT NOT NULL,
    "last_used" BIGINT,
    "last_played" BIGINT
);

INSERT INTO "new_tracks" SELECT * FROM "tracks";
DROP TABLE "tracks";
ALTER TABLE "new_tracks" RENAME TO "tracks";
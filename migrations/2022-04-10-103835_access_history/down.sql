ALTER TABLE "persons" DROP COLUMN "last_used";
ALTER TABLE "persons" DROP COLUMN "last_played";

ALTER TABLE "instruments" DROP COLUMN "last_used";
ALTER TABLE "instruments" DROP COLUMN "last_played";

ALTER TABLE "works" DROP COLUMN "last_used";
ALTER TABLE "works" DROP COLUMN "last_played";

ALTER TABLE "ensembles" DROP COLUMN "last_used";
ALTER TABLE "ensembles" DROP COLUMN "last_played";

ALTER TABLE "recordings" DROP COLUMN "last_used";
ALTER TABLE "recordings" DROP COLUMN "last_played";

ALTER TABLE "mediums" DROP COLUMN "last_used";
ALTER TABLE "mediums" DROP COLUMN "last_played";

ALTER TABLE "tracks" DROP COLUMN "last_used";
ALTER TABLE "tracks" DROP COLUMN "last_played";

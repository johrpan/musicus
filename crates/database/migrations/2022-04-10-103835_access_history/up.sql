ALTER TABLE "persons" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "persons" ADD COLUMN "last_played" BIGINT;

ALTER TABLE "instruments" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "instruments" ADD COLUMN "last_played" BIGINT;

ALTER TABLE "works" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "works" ADD COLUMN "last_played" BIGINT;

ALTER TABLE "ensembles" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "ensembles" ADD COLUMN "last_played" BIGINT;

ALTER TABLE "recordings" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "recordings" ADD COLUMN "last_played" BIGINT;

ALTER TABLE "mediums" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "mediums" ADD COLUMN "last_played" BIGINT;

ALTER TABLE "tracks" ADD COLUMN "last_used" BIGINT;
ALTER TABLE "tracks" ADD COLUMN "last_played" BIGINT;


//! Procedural generation of plausible looking names.
//!
//! Everything here is invented: the generators assemble names from syllable and
//! word tables rather than drawing from a catalogue of real people, ensembles or
//! compositions. The point is that the result *looks* like classical music
//! metadata at a glance, not that it is accurate.
//!
//! Each generator hands out unique names within one [`Names`] instance. When a
//! combinator has exhausted its space, a numeric suffix keeps the names unique
//! rather than the generator looping forever.

use std::collections::HashSet;

use rand::prelude::*;

const GIVEN_NAME_STARTS: &[&str] = &[
    "Al", "An", "Ar", "Aug", "Bern", "Cas", "Cor", "Dan", "Ed", "El", "Er", "Fer", "Fried", "Gab",
    "Ger", "Hel", "Hen", "Ig", "Il", "Jo", "Kas", "Kon", "Lav", "Leo", "Lud", "Mar", "Mat", "Mir",
    "Nad", "Nik", "Ot", "Pav", "Rein", "Rom", "Sev", "Sil", "Ste", "Ther", "Val", "Vin", "Wil",
    "Zol",
];

const GIVEN_NAME_ENDS: &[&str] = &[
    "a", "an", "as", "ard", "beth", "el", "en", "ena", "ette", "hild", "ia", "ian", "ika", "ilde",
    "ina", "ine", "is", "ius", "o", "old", "on", "ora", "rik", "sander", "sim", "tav", "tor",
    "vard", "wig",
];

const SURNAME_STARTS: &[&str] = &[
    "Ass", "Bell", "Bran", "Car", "Del", "Dre", "Falk", "Gar", "Grin", "Hall", "Hart", "Jans",
    "Kal", "Kar", "Kle", "Lam", "Lind", "Mar", "Mel", "Mor", "Nov", "Ost", "Pel", "Pra", "Rein",
    "Rou", "Sal", "Schwar", "Ser", "Stein", "Tar", "Thal", "Vas", "Ver", "Wald", "Wen", "Zab",
    "Zim",
];

const SURNAME_MIDDLES: &[&str] = &[
    "", "", "", "ber", "der", "en", "in", "kow", "lo", "man", "ner", "sen", "ta", "ven",
];

const SURNAME_ENDS: &[&str] = &[
    "ay", "berg", "court", "dahl", "elli", "esco", "feld", "hardt", "ini", "ius", "kin", "mont",
    "nen", "off", "oux", "sky", "son", "stad", "thal", "ucci", "vari", "witz", "yev",
];

const SURNAME_PARTICLES: &[&str] = &["van", "von", "de", "di", "del", "da"];

const INSTRUMENT_MODIFIERS: &[&str] = &[
    "",
    "",
    "",
    "Alto",
    "Baritone",
    "Bass",
    "Contrabass",
    "Descant",
    "Great",
    "Piccolo",
    "Soprano",
    "Tenor",
    "Treble",
];

const INSTRUMENT_STEMS: &[&str] = &[
    "Cithern",
    "Clarion",
    "Cornett",
    "Crembalum",
    "Dulcian",
    "Fidula",
    "Flageolet",
    "Gemshorn",
    "Hurdy",
    "Kantele",
    "Lirone",
    "Lute",
    "Nyckelharp",
    "Ocarina",
    "Psaltery",
    "Rebec",
    "Sackbut",
    "Serpent",
    "Shawm",
    "Theorbo",
    "Tromba",
    "Viol",
    "Virginal",
    "Zink",
];

const ROLE_WORDS: &[&str] = &[
    "Conductor",
    "Soloist",
    "Chorus Master",
    "Continuo",
    "Concertmaster",
    "Arranger",
    "Librettist",
    "Improviser",
    "Repetiteur",
    "Narrator",
];

const PLACE_STARTS: &[&str] = &[
    "Aal", "Bres", "Carn", "Dorn", "Elm", "Falk", "Gran", "Hoch", "Iber", "Kron", "Lav", "Mar",
    "Nord", "Ost", "Pren", "Rhen", "Sal", "Thorn", "Ulm", "Vester", "Wester", "Zerb",
];

const PLACE_ENDS: &[&str] = &[
    "bach", "borg", "bruck", "burg", "dorf", "feld", "furt", "hausen", "heim", "holm", "mund",
    "stad", "stein", "tal", "vik", "wald",
];

const ENSEMBLE_TYPES: &[&str] = &[
    "Academy",
    "Camerata",
    "Chamber Orchestra",
    "Chamber Players",
    "Collegium",
    "Consort",
    "Ensemble",
    "Festival Orchestra",
    "Philharmonic",
    "Quartet",
    "Quintet",
    "Sinfonietta",
    "String Trio",
    "Symphony Orchestra",
    "Vocal Ensemble",
];

const WORK_FORMS: &[&str] = &[
    "Symphony",
    "Sonata",
    "Concerto",
    "Suite",
    "Quartet",
    "Quintet",
    "Trio",
    "Overture",
    "Fantasia",
    "Rhapsody",
    "Serenade",
    "Divertimento",
    "Prelude",
    "Nocturne",
    "Variations",
    "Cantata",
];

const KEYS: &[&str] = &[
    "C major",
    "C minor",
    "D major",
    "D minor",
    "E-flat major",
    "E minor",
    "F major",
    "F-sharp minor",
    "G major",
    "G minor",
    "A major",
    "A minor",
    "B-flat major",
    "B minor",
];

const EPITHETS: &[&str] = &[
    "Autumnal",
    "Bell",
    "Hunting",
    "Lantern",
    "Northern",
    "Pastoral",
    "Pilgrim",
    "Reformation",
    "Shepherd",
    "Solstice",
    "Tempest",
    "Twilight",
    "Wanderer",
    "Winter",
];

/// A seeded source of unique, invented names.
pub struct Names {
    rng: StdRng,
    used: HashSet<String>,
}

impl Names {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            used: HashSet::new(),
        }
    }

    /// The random number generator backing the name tables.
    ///
    /// The generator shares it so that the whole run stays reproducible from the
    /// single seed.
    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// A person's name, in "given name surname" form.
    pub fn person(&mut self) -> String {
        self.unique(|rng| {
            let given = format!(
                "{}{}",
                pick(rng, GIVEN_NAME_STARTS),
                pick(rng, GIVEN_NAME_ENDS)
            );
            let family = surname(rng);

            match rng.random_range(0..10) {
                0 => format!("{given} {} {family}", pick(rng, SURNAME_PARTICLES)),
                1 => format!("{given} {family}-{}", surname(rng)),
                _ => format!("{given} {family}"),
            }
        })
    }

    /// An instrument name.
    pub fn instrument(&mut self) -> String {
        self.unique(|rng| {
            let modifier = pick(rng, INSTRUMENT_MODIFIERS);
            let stem = pick(rng, INSTRUMENT_STEMS);

            if modifier.is_empty() {
                stem.to_string()
            } else {
                format!("{modifier} {stem}")
            }
        })
    }

    /// A performer role.
    pub fn role(&mut self) -> String {
        self.unique(|rng| pick(rng, ROLE_WORDS).to_string())
    }

    /// An ensemble name, built from an invented place and an ensemble type.
    pub fn ensemble(&mut self) -> String {
        self.unique(|rng| {
            let place = format!("{}{}", pick(rng, PLACE_STARTS), pick(rng, PLACE_ENDS));
            let kind = pick(rng, ENSEMBLE_TYPES);

            match rng.random_range(0..8) {
                0 => format!("{kind} of {place}"),
                1 => format!("New {place} {kind}"),
                _ => format!("{place} {kind}"),
            }
        })
    }

    /// A work title, in the shape of a catalogue entry.
    pub fn work(&mut self) -> String {
        self.unique(|rng| {
            let form = pick(rng, WORK_FORMS);
            let mut title = form.to_string();

            if rng.random_bool(0.7) {
                title.push_str(&format!(" No. {}", rng.random_range(1..13)));
            }

            if rng.random_bool(0.6) {
                title.push_str(&format!(" in {}", pick(rng, KEYS)));
            }

            if rng.random_bool(0.5) {
                title.push_str(&format!(", Op. {}", rng.random_range(1..131)));
            }

            if rng.random_bool(0.2) {
                title.push_str(&format!(" \"{}\"", pick(rng, EPITHETS)));
            }

            title
        })
    }

    /// Draw from `generate` until it produces a name not yet handed out.
    ///
    /// After a number of collisions the combinator's space is assumed to be
    /// crowded and a counter is appended instead, so that a run asking for more
    /// names than the tables can produce still terminates.
    fn unique(&mut self, generate: impl Fn(&mut StdRng) -> String) -> String {
        const ATTEMPTS: usize = 32;

        for _ in 0..ATTEMPTS {
            let name = generate(&mut self.rng);

            if self.used.insert(name.clone()) {
                return name;
            }
        }

        let base = generate(&mut self.rng);

        for suffix in 2.. {
            let name = format!("{base} {suffix}");

            if self.used.insert(name.clone()) {
                return name;
            }
        }

        unreachable!("the suffix counter is unbounded")
    }
}

fn pick<'a>(rng: &mut StdRng, words: &[&'a str]) -> &'a str {
    words.choose(rng).expect("word tables are never empty")
}

fn surname(rng: &mut StdRng) -> String {
    format!(
        "{}{}{}",
        pick(rng, SURNAME_STARTS),
        pick(rng, SURNAME_MIDDLES),
        pick(rng, SURNAME_ENDS)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_beyond_the_table_size() {
        let mut names = Names::new(1);

        // More roles than there are role words, so the suffix fallback is used.
        let roles = (0..ROLE_WORDS.len() * 3)
            .map(|_| names.role())
            .collect::<HashSet<String>>();

        assert_eq!(roles.len(), ROLE_WORDS.len() * 3);
    }

    #[test]
    fn the_same_seed_produces_the_same_names() {
        let generate = |seed| {
            let mut names = Names::new(seed);
            (0..50).map(|_| names.person()).collect::<Vec<String>>()
        };

        assert_eq!(generate(42), generate(42));
        assert_ne!(generate(42), generate(43));
    }

    #[test]
    fn every_generator_produces_a_non_empty_name() {
        let mut names = Names::new(7);

        for _ in 0..100 {
            assert!(!names.person().is_empty());
            assert!(!names.instrument().is_empty());
            assert!(!names.role().is_empty());
            assert!(!names.ensemble().is_empty());
            assert!(!names.work().is_empty());
        }
    }
}

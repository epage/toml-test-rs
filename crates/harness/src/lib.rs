#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

use std::io::Write;

pub use toml_test::decoded::Decoded;
pub use toml_test::decoded::DecodedValue;
pub use toml_test::verify::Decoder;
pub use toml_test::verify::Encoder;
pub use toml_test::Error;

pub struct DecoderHarness<D> {
    decoder: D,
    matches: Option<Matches>,
    version: Option<String>,
}

impl<D> DecoderHarness<D>
where
    D: Decoder + Copy + Send + Sync + 'static,
{
    pub fn new(decoder: D) -> Self {
        Self {
            decoder,
            matches: None,
            version: None,
        }
    }

    pub fn ignore<'p>(
        &mut self,
        patterns: impl IntoIterator<Item = &'p str>,
    ) -> Result<&mut Self, Error> {
        self.matches = Some(Matches::new(patterns.into_iter())?);
        Ok(self)
    }

    pub fn version(&mut self, version: impl Into<String>) -> &mut Self {
        self.version = Some(version.into());
        self
    }

    pub fn test(self) -> ! {
        let args = libtest_mimic::Arguments::from_args();
        let nocapture = args.nocapture;

        let versioned = self
            .version
            .as_deref()
            .into_iter()
            .flat_map(toml_test_data::version)
            .collect::<std::collections::HashSet<_>>();

        let mut tests = Vec::new();
        let decoder = self.decoder;
        tests.extend(
            toml_test_data::valid()
                .map(|case| {
                    let ignore = self
                        .matches
                        .as_ref()
                        .map(|m| !m.matched(case.name))
                        .unwrap_or_default()
                        || !versioned.contains(case.name);
                    (case, ignore)
                })
                .map(move |(case, ignore)| {
                    libtest_mimic::Trial::test(case.name.display().to_string(), move || {
                        decoder
                            .verify_valid_case(case.fixture, case.expected)
                            .map_err(libtest_mimic::Failed::from)
                    })
                    .with_ignored_flag(ignore)
                }),
        );
        tests.extend(
            toml_test_data::invalid()
                .map(|case| {
                    let ignore = self
                        .matches
                        .as_ref()
                        .map(|m| !m.matched(case.name))
                        .unwrap_or_default()
                        || !versioned.contains(case.name);
                    (case, ignore)
                })
                .map(move |(case, ignore)| {
                    libtest_mimic::Trial::test(case.name.display().to_string(), move || {
                        match decoder.verify_invalid_case(case.fixture) {
                            Ok(err) => {
                                if nocapture {
                                    let _ = writeln!(std::io::stdout(), "{err}");
                                }
                                Ok(())
                            }
                            Err(err) => Err(libtest_mimic::Failed::from(err)),
                        }
                    })
                    .with_ignored_flag(ignore)
                }),
        );

        libtest_mimic::run(&args, tests).exit()
    }
}

pub struct EncoderHarness<E, D> {
    encoder: E,
    fixture: D,
    matches: Option<Matches>,
    version: Option<String>,
}

impl<E, D> EncoderHarness<E, D>
where
    E: Encoder + Copy + Send + Sync + 'static,
    D: Decoder + Copy + Send + Sync + 'static,
{
    pub fn new(encoder: E, fixture: D) -> Self {
        Self {
            encoder,
            fixture,
            matches: None,
            version: None,
        }
    }

    pub fn ignore<'p>(
        &mut self,
        patterns: impl IntoIterator<Item = &'p str>,
    ) -> Result<&mut Self, Error> {
        self.matches = Some(Matches::new(patterns.into_iter())?);
        Ok(self)
    }

    pub fn version(&mut self, version: impl Into<String>) -> &mut Self {
        self.version = Some(version.into());
        self
    }

    pub fn test(self) -> ! {
        let args = libtest_mimic::Arguments::from_args();

        let versioned = self
            .version
            .as_deref()
            .into_iter()
            .flat_map(toml_test_data::version)
            .collect::<std::collections::HashSet<_>>();

        let mut tests = Vec::new();
        let encoder = self.encoder;
        let fixture = self.fixture;
        tests.extend(
            toml_test_data::valid()
                .map(|case| {
                    let ignore = self
                        .matches
                        .as_ref()
                        .map(|m| !m.matched(case.name))
                        .unwrap_or_default()
                        || !versioned.contains(case.name);
                    (case, ignore)
                })
                .map(move |(case, ignore)| {
                    libtest_mimic::Trial::test(case.name.display().to_string(), move || {
                        encoder
                            .verify_valid_case(case.expected, &fixture)
                            .map_err(libtest_mimic::Failed::from)
                    })
                    .with_ignored_flag(ignore)
                }),
        );
        libtest_mimic::run(&args, tests).exit()
    }
}

struct Matches {
    ignores: ignore::gitignore::Gitignore,
}

impl Matches {
    fn new<'p>(patterns: impl Iterator<Item = &'p str>) -> Result<Self, Error> {
        let mut ignores = ignore::gitignore::GitignoreBuilder::new(".");
        for line in patterns {
            ignores.add_line(None, line).map_err(Error::new)?;
        }
        let ignores = ignores.build().map_err(Error::new)?;
        Ok(Self { ignores })
    }

    fn matched(&self, path: &std::path::Path) -> bool {
        match self.ignores.matched_path_or_any_parents(path, false) {
            ignore::Match::None | ignore::Match::Whitelist(_) => true,
            ignore::Match::Ignore(_) => false,
        }
    }
}

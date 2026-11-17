mod common;

use lita_tokenizers_spacy::{Error, Spacy, TokenAttribute, TokenAttributes};
use tokenizers_core::{Token, TokenPosition, TokenStream, TokenStreamBuilder, Tokenizer};

type TokenRecord = (TokenPosition, String, Vec<String>);

fn snapshot(stream: &impl TokenStream) -> Vec<TokenRecord> {
    stream
        .tokens()
        .map(|(position, token)| {
            (
                position,
                token.surface().as_str().to_owned(),
                token.tags().map(|tag| tag.as_str().to_owned()).collect(),
            )
        })
        .collect()
}

fn tokenize(
    tokenizer: &mut Spacy<impl TokenAttributes>,
    input: &str,
) -> Result<Vec<TokenRecord>, Error> {
    let mut builder = tokenizer.builder()?;
    let stream = builder.tokenize(input)?;
    Ok(snapshot(&stream))
}

#[test]
fn english_contractions_abbreviations_and_sentences() -> Result<(), Error> {
    use TokenPosition::{Eos, Normal};

    let mut tokenizer = Spacy::from(common::environment().blank_english()?);
    let tokens = tokenize(
        &mut tokenizer,
        "Mr. Smith can't visit the U.K. today. He'll call tomorrow!",
    )?;

    let expected = [
        (Normal, "Mr."),
        (Normal, "Smith"),
        (Normal, "ca"),
        (Normal, "n't"),
        (Normal, "visit"),
        (Normal, "the"),
        (Normal, "U.K."),
        (Normal, "today"),
        (Eos, "."),
        (Normal, "He"),
        (Normal, "'ll"),
        (Normal, "call"),
        (Normal, "tomorrow"),
        (Eos, "!"),
    ];
    assert_eq!(
        tokens
            .iter()
            .map(|(position, surface, _)| (*position, surface.as_str()))
            .collect::<Vec<_>>(),
        expected,
    );
    for (_, _, tags) in tokens {
        assert_eq!(tags, ["", "", "", ""]);
    }
    Ok(())
}

#[test]
fn whitespace_is_omitted_without_losing_sentence_ends() -> Result<(), Error> {
    use TokenPosition::{Eos, Normal};

    let mut tokenizer = Spacy::from(common::environment().blank_english()?);
    let tokens = tokenize(&mut tokenizer, " \tHello world. \n\nGoodbye for now \t")?;
    assert_eq!(
        tokens
            .iter()
            .map(|(position, surface, _)| (*position, surface.as_str()))
            .collect::<Vec<_>>(),
        [
            (Normal, "Hello"),
            (Normal, "world"),
            (Eos, "."),
            (Normal, "Goodbye"),
            (Normal, "for"),
            (Eos, "now"),
        ],
    );
    Ok(())
}

#[test]
fn unicode_surfaces_are_preserved() -> Result<(), Error> {
    let mut tokenizer = Spacy::from(common::environment().blank_english()?);
    let tokens = tokenize(&mut tokenizer, "Café owners aren't naïve.")?;
    assert_eq!(
        tokens
            .iter()
            .map(|(_, surface, _)| surface.as_str())
            .collect::<Vec<_>>(),
        ["Café", "owners", "are", "n't", "naïve", "."],
    );
    assert_eq!(tokens.last().unwrap().0, TokenPosition::Eos);
    Ok(())
}

#[test]
fn empty_and_whitespace_only_inputs_have_no_tokens() -> Result<(), Error> {
    let mut tokenizer = Spacy::from(common::environment().blank_english()?);
    for input in ["", " ", "\t\n \n"] {
        assert!(tokenize(&mut tokenizer, input)?.is_empty(), "{input:?}");
    }
    Ok(())
}

#[test]
fn streams_survive_pipeline_reuse_and_dropped_inputs_and_tokenizers() -> Result<(), Error> {
    let stream = {
        let mut tokenizer = Spacy::from(common::environment().blank_english()?);
        let input = String::from("Hello, world!");
        let mut builder = tokenizer.builder()?;
        let stream = builder.tokenize(&input)?;

        let next_stream = builder.tokenize("Goodbye.")?;
        assert_eq!(
            snapshot(&next_stream)
                .iter()
                .map(|(_, surface, _)| surface.as_str())
                .collect::<Vec<_>>(),
            ["Goodbye", "."],
        );
        stream
    };

    let tokens = snapshot(&stream);
    assert_eq!(
        tokens
            .iter()
            .map(|(_, surface, _)| surface.as_str())
            .collect::<Vec<_>>(),
        ["Hello", ",", "world", "!"],
    );
    assert_eq!(tokens.last().unwrap().0, TokenPosition::Eos);
    assert_eq!(snapshot(&stream), tokens);
    Ok(())
}

#[test]
fn missing_model_returns_a_python_error() -> Result<(), Error> {
    // Ensure an import failure cannot accidentally satisfy the missing-model assertion.
    let _tokenizer = Spacy::from(common::environment().blank_english()?);
    let model = "__lita_tokenizers_spacy_missing_model__";
    let error = common::environment()
        .config()
        .load(model)
        .err()
        .expect("loading a nonexistent model must fail");
    assert!(error.to_string().contains(model), "{error}");
    Ok(())
}

#[test]
fn trained_english_pipeline_supplies_linguistic_tags() -> Result<(), Error> {
    let mut tokenizer = Spacy::from(common::environment().config().load("en_core_web_sm")?);
    let tokens = tokenize(&mut tokenizer, "We are programmers.")?;
    assert_eq!(tokens.len(), 4);
    for (index, (surface, pos, fine_pos, lemma)) in [
        ("We", "PRON", "PRP", "we"),
        ("are", "AUX", "VBP", "be"),
        ("programmers", "NOUN", "NNS", "programmer"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(tokens[index].1, surface);
        let tags = &tokens[index].2;
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[0], pos);
        assert_eq!(tags[1], fine_pos);
        assert_eq!(tags[2], lemma);
    }
    assert!(tokens[2].2[3].split('|').any(|tag| tag == "Number=Plur"));
    assert_eq!(tokens[3].1, ".");
    assert_eq!(tokens[3].0, TokenPosition::Eos);
    Ok(())
}

#[test]
fn token_attributes_specify_the_core_tag_values() -> Result<(), Error> {
    use TokenAttribute::{Lemma, Pos, Tag};
    let model = common::environment().config().load("en_core_web_sm")?;
    let attributes = [Lemma, Pos, Lemma, Tag];
    let mut tokenizer = Spacy::new(model, attributes);
    assert_eq!(
        tokenizer.attributes().collect::<Vec<_>>(),
        [Lemma, Pos, Lemma, Tag]
    );
    let tokens = tokenize(&mut tokenizer, "Programmers.")?;
    assert_eq!(tokens[0].1, "Programmers");
    assert_eq!(tokens[0].2, ["programmer", "NOUN", "programmer", "NNS"]);

    Ok(())
}

#[test]
fn empty_attribute_selection_omits_all_tags() -> Result<(), Error> {
    let model = common::environment().blank_english()?;
    let mut tokenizer = Spacy::new(model, Vec::new());
    assert_eq!(
        tokenize(&mut tokenizer, "Hello.")?,
        [
            (TokenPosition::Normal, "Hello".to_owned(), vec![]),
            (TokenPosition::Eos, ".".to_owned(), vec![]),
        ],
    );
    Ok(())
}

#[test]
fn runtime_attribute_counts_retain_token_and_sentence_alignment() -> Result<(), Error> {
    use TokenAttribute::{Lemma, Morph, Pos, Tag};

    for count in [0, 1, 2, 4, 9] {
        let attributes = [Lemma, Pos, Tag, Morph]
            .into_iter()
            .cycle()
            .take(count)
            .collect::<Vec<_>>();
        let model = common::environment().config().load("en_core_web_sm")?;
        let mut tokenizer = Spacy::new(model, attributes);
        let expected_noun = ["programmer", "NOUN", "NNS", "Number=Plur"]
            .into_iter()
            .cycle()
            .take(count)
            .map(String::from)
            .collect::<Vec<_>>();
        let expected_punctuation = [".", "PUNCT", ".", "PunctType=Peri"]
            .into_iter()
            .cycle()
            .take(count)
            .map(String::from)
            .collect::<Vec<_>>();
        assert_eq!(
            tokenize(&mut tokenizer, "Programmers. Programmers.")?,
            [
                (
                    TokenPosition::Normal,
                    "Programmers".to_owned(),
                    expected_noun.clone()
                ),
                (
                    TokenPosition::Eos,
                    ".".to_owned(),
                    expected_punctuation.clone()
                ),
                (
                    TokenPosition::Normal,
                    "Programmers".to_owned(),
                    expected_noun
                ),
                (TokenPosition::Eos, ".".to_owned(), expected_punctuation),
            ],
            "attribute count {count}",
        );
    }
    Ok(())
}

#[test]
fn selected_unannotated_attributes_have_empty_values() -> Result<(), Error> {
    let model = common::environment().blank_english()?;
    let mut tokenizer = Spacy::new(model, vec![TokenAttribute::Lemma, TokenAttribute::Morph]);
    assert_eq!(
        tokenize(&mut tokenizer, "Hello.")?,
        [
            (
                TokenPosition::Normal,
                "Hello".to_owned(),
                vec!["".to_owned(), "".to_owned()]
            ),
            (
                TokenPosition::Eos,
                ".".to_owned(),
                vec!["".to_owned(), "".to_owned()]
            ),
        ]
    );
    Ok(())
}

#[test]
fn stream_outlives_its_borrowed_attribute_source() -> Result<(), Error> {
    let stream = {
        let attributes = vec![TokenAttribute::Lemma, TokenAttribute::Pos];
        let model = common::environment().config().load("en_core_web_sm")?;
        let mut tokenizer = Spacy::new(model, &attributes);
        let mut builder = tokenizer.builder()?;
        builder.tokenize("Programmers.")?
    };

    assert_eq!(
        snapshot(&stream),
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec!["programmer".to_owned(), "NOUN".to_owned()]
            ),
            (
                TokenPosition::Eos,
                ".".to_owned(),
                vec![".".to_owned(), "PUNCT".to_owned()]
            ),
        ],
    );
    Ok(())
}

#[test]
fn owning_attribute_source_supplies_morphology_and_tag() -> Result<(), Error> {
    let attributes = vec![TokenAttribute::Morph, TokenAttribute::Tag];
    let model = common::environment().config().load("en_core_web_sm")?;
    let mut tokenizer = Spacy::new(model, attributes);
    let mut builder = tokenizer.builder()?;
    let stream = builder.tokenize("Programmers.")?;
    assert_eq!(
        snapshot(&stream),
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec!["Number=Plur".to_owned(), "NNS".to_owned()]
            ),
            (
                TokenPosition::Eos,
                ".".to_owned(),
                vec!["PunctType=Peri".to_owned(), ".".to_owned()]
            ),
        ],
    );
    Ok(())
}

#[test]
fn custom_attribute_source_defines_its_extraction_specification() -> Result<(), Error> {
    struct Attributes([TokenAttribute; 2]);

    impl TokenAttributes for Attributes {
        type Iter<'a> = std::iter::Copied<std::slice::Iter<'a, TokenAttribute>>;

        fn attributes(&self) -> Self::Iter<'_> {
            self.0.iter().copied()
        }
    }

    let stream = {
        let model = common::environment().config().load("en_core_web_sm")?;
        let mut tokenizer = Spacy::new(
            model,
            Attributes([TokenAttribute::Tag, TokenAttribute::Pos]),
        );
        let mut builder = tokenizer.builder()?;
        builder.tokenize("Programmers.")?
    };
    assert_eq!(
        snapshot(&stream),
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec!["NNS".to_owned(), "NOUN".to_owned()]
            ),
            (
                TokenPosition::Eos,
                ".".to_owned(),
                vec![".".to_owned(), "PUNCT".to_owned()]
            ),
        ],
    );
    Ok(())
}

#[test]
fn builder_uses_current_specification_for_each_enumeration() -> Result<(), Error> {
    use std::cell::Cell;

    struct Attributes<'a>(&'a Cell<bool>);

    impl TokenAttributes for Attributes<'_> {
        type Iter<'a>
            =
            std::iter::Chain<std::iter::Once<TokenAttribute>, std::option::IntoIter<TokenAttribute>>
        where
            Self: 'a;

        fn attributes(&self) -> Self::Iter<'_> {
            let (first, second) = if self.0.get() {
                (TokenAttribute::Lemma, None)
            } else {
                (TokenAttribute::Tag, Some(TokenAttribute::Pos))
            };
            std::iter::once(first).chain(second)
        }
    }

    let lemma = Cell::new(false);
    let model = common::environment().config().load("en_core_web_sm")?;
    let mut attributes = Attributes(&lemma);
    let mut tokenizer = Spacy::new(model, &mut attributes);
    let (first, second) = {
        let mut builder = tokenizer.builder()?;
        let first = builder.tokenize("Programmers.")?;
        lemma.set(true);
        let second = builder.tokenize("Programmers.")?;
        (first, second)
    };
    let first_snapshot = snapshot(&first);
    assert_eq!(
        first_snapshot,
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec!["NNS".to_owned(), "NOUN".to_owned()]
            ),
            (
                TokenPosition::Eos,
                ".".to_owned(),
                vec![".".to_owned(), "PUNCT".to_owned()]
            ),
        ]
    );
    assert_eq!(
        snapshot(&second),
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec!["programmer".to_owned()]
            ),
            (TokenPosition::Eos, ".".to_owned(), vec![".".to_owned()]),
        ]
    );
    let mut builder = tokenizer.builder()?;
    let third = builder.tokenize("Programmers.")?;
    assert_eq!(
        snapshot(&third),
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec!["programmer".to_owned()]
            ),
            (TokenPosition::Eos, ".".to_owned(), vec![".".to_owned()]),
        ]
    );
    assert_eq!(snapshot(&third), snapshot(&second));
    assert_eq!(snapshot(&first), first_snapshot);
    Ok(())
}

#[test]
fn missing_sentence_boundaries_fail_during_tokenization() -> Result<(), Error> {
    let model = common::environment().config().blank("en")?;
    let mut tokenizer = Spacy::from(model);
    let mut builder = tokenizer.builder()?;
    let error = builder
        .tokenize("A sentence.")
        .err()
        .expect("sentence boundaries are required");
    assert!(error.to_string().contains("E030"), "{error}");
    Ok(())
}

#[test]
fn token_iterator_is_fused_and_reports_remaining_length() -> Result<(), Error> {
    let mut tokenizer = Spacy::from(common::environment().blank_english()?);
    let mut builder = tokenizer.builder()?;
    let stream = builder.tokenize("Hi.")?;
    let mut tokens = stream.tokens();
    assert_eq!(tokens.size_hint(), (2, Some(2)));
    let first = tokens.next().unwrap().1;
    assert_eq!(first.surface().as_str(), "Hi");
    let mut tags = first.tags();
    assert_eq!(tags.len(), 4);
    assert_eq!(
        tags.by_ref().map(|tag| tag.as_str()).collect::<Vec<_>>(),
        ["", "", "", ""]
    );
    assert_eq!(tags.len(), 0);
    assert_eq!(tags.next(), None);
    assert_eq!(tags.next(), None);
    assert_eq!(tokens.size_hint(), (1, Some(1)));
    assert_eq!(tokens.next().unwrap().0, TokenPosition::Eos);
    assert!(tokens.next().is_none());
    assert!(tokens.next().is_none());
    assert_eq!(tokens.size_hint(), (0, Some(0)));
    Ok(())
}

#[test]
fn lazy_attribute_iterator_computes_filters_and_repeats_values() -> Result<(), Error> {
    use lita_tokenizers_spacy::TokenAttributes;

    struct Attributes {
        fine_pos: bool,
    }

    struct AttributesIter<'a> {
        source: &'a Attributes,
        stage: usize,
    }

    impl Iterator for AttributesIter<'_> {
        type Item = TokenAttribute;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                let attribute = match self.stage {
                    0 | 3 => TokenAttribute::Lemma,
                    1 => TokenAttribute::Tag,
                    2 => TokenAttribute::Pos,
                    _ => return None,
                };
                self.stage += 1;
                if attribute == TokenAttribute::Tag && !self.source.fine_pos {
                    continue;
                }
                return Some(attribute);
            }
        }
    }

    impl TokenAttributes for Attributes {
        type Iter<'a> = AttributesIter<'a>;

        fn attributes(&self) -> Self::Iter<'_> {
            AttributesIter {
                source: self,
                stage: 0,
            }
        }
    }

    let stream = {
        let attributes = Attributes { fine_pos: false };
        let model = common::environment().config().load("en_core_web_sm")?;
        let mut tokenizer = Spacy::new(model, &attributes);
        assert_eq!(tokenizer.attributes().size_hint(), (0, None));
        let mut builder = tokenizer.builder()?;
        builder.tokenize("Programmers.")?
    };
    assert_eq!(
        snapshot(&stream),
        [
            (
                TokenPosition::Normal,
                "Programmers".to_owned(),
                vec![
                    "programmer".to_owned(),
                    "NOUN".to_owned(),
                    "programmer".to_owned()
                ]
            ),
            (
                TokenPosition::Eos,
                ".".to_owned(),
                vec![".".to_owned(), "PUNCT".to_owned(), ".".to_owned()]
            ),
        ],
    );
    Ok(())
}

#[test]
fn per_token_attribute_counts_follow_each_enumeration() -> Result<(), Error> {
    use lita_tokenizers_spacy::TokenAttributes;
    use std::cell::Cell;
    use std::iter::Take;

    struct Attributes(Cell<usize>);

    impl TokenAttributes for Attributes {
        type Iter<'a> = Take<std::array::IntoIter<TokenAttribute, 2>>;

        fn attributes(&self) -> Self::Iter<'_> {
            let n = self.0.get();
            self.0.set(n + 1);
            let phase = n % 3;
            let attributes = match phase {
                0 => [TokenAttribute::Morph, TokenAttribute::Tag],
                1 => [TokenAttribute::Pos, TokenAttribute::Lemma],
                _ => [TokenAttribute::Tag, TokenAttribute::Lemma],
            };
            attributes.into_iter().take(phase)
        }
    }

    let stream = {
        let attributes = Attributes(Cell::new(0));
        let model = common::environment().config().load("en_core_web_sm")?;
        let mut tokenizer = Spacy::new(model, &attributes);
        let mut builder = tokenizer.builder()?;
        let stream = builder.tokenize("Programmers. Programmers. Programmers.")?;
        assert_eq!(attributes.0.get(), 6);
        stream
    };
    let expected = [
        (TokenPosition::Normal, "Programmers".to_owned(), vec![]),
        (TokenPosition::Eos, ".".to_owned(), vec!["PUNCT".to_owned()]),
        (
            TokenPosition::Normal,
            "Programmers".to_owned(),
            vec!["NNS".to_owned(), "programmer".to_owned()],
        ),
        (TokenPosition::Eos, ".".to_owned(), vec![]),
        (
            TokenPosition::Normal,
            "Programmers".to_owned(),
            vec!["NOUN".to_owned()],
        ),
        (
            TokenPosition::Eos,
            ".".to_owned(),
            vec![".".to_owned(), ".".to_owned()],
        ),
    ];
    assert_eq!(snapshot(&stream), expected);
    assert_eq!(snapshot(&stream), expected);
    let mut tokens = stream.tokens();
    for _ in &expected {
        assert!(tokens.next().is_some());
    }
    assert!(tokens.next().is_none());
    assert!(tokens.next().is_none());
    assert_eq!(tokens.size_hint(), (0, Some(0)));
    Ok(())
}

use super::*;

const KYTEA_MODEL_BIN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/models/model.bin"
));
const KYTEA_MODEL_TXT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/models/model.txt"
));

#[test]
fn correct_version() {
    assert_eq!(version(), "0.4.8");
}

#[test]
fn tokenize_bin() {
    let mut model = KyTea::new();

    model
        .config()
        .set_debug(DebugLevel::Silent)
        .set_training(false)
        .set_word_bound(c" ")
        .set_input_format(CorpusFormat::Raw);

    let model_content = KYTEA_MODEL_BIN.to_vec();
    let mut model_stream = IspanStream::new(&model_content);

    model
        .read_model_from_stream(&mut model_stream, ModelFormat::Binary)
        .unwrap();
    drop(model_stream);
    drop(model_content);

    let mut input = StringStream::new();
    input.push("すもももももももものうち．\n");
    input.push("すもももももももものうち．\n");
    input.push("すもももももももものうち．\n");

    let mut output = StringStream::new();

    let mut ctx = model.context(&mut input, &mut output).unwrap();
    while ctx.predict().unwrap().is_continue() {}

    let bytes = output.as_bytes();
    let expected = [
        "すもも/名詞/すもも",
        "も/助詞/も",
        "もも/名詞/もも",
        "も/助詞/も",
        "もも/名詞/もも",
        "の/助詞/の",
        "うち/名詞/うち",
        "．/補助記号/。\n",
    ]
    .join(" ")
    .repeat(3);

    assert_eq!(
        bytes,
        expected.as_bytes(),
        "prediction failed: {}",
        String::from_utf8_lossy(bytes)
    );
}

#[test]
fn tokenize_txt() {
    let mut model = KyTea::new();

    model
        .config()
        .set_debug(DebugLevel::Silent)
        .set_training(false)
        .set_word_bound(c" ")
        .set_input_format(CorpusFormat::Raw);

    let model_content = KYTEA_MODEL_TXT.to_vec();
    let mut model_stream = IspanStream::new(&model_content);

    model
        .read_model_from_stream(&mut model_stream, ModelFormat::Text)
        .unwrap();
    drop(model_stream);
    drop(model_content);

    let mut input = StringStream::new();
    input.push("すもももももももものうち．\n");
    input.push("すもももももももものうち．\n");
    input.push("すもももももももものうち．\n");

    let mut output = StringStream::new();

    let mut ctx = model.context(&mut input, &mut output).unwrap();
    while ctx.predict().unwrap().is_continue() {}

    let bytes = output.as_bytes();
    let expected = [
        "すもも/名詞/すもも",
        "も/助詞/も",
        "もも/名詞/もも",
        "も/助詞/も",
        "もも/名詞/もも",
        "の/助詞/の",
        "うち/名詞/うち",
        "．/補助記号/。\n",
    ]
    .join(" ")
    .repeat(3);

    assert_eq!(
        bytes,
        expected.as_bytes(),
        "prediction failed: {}",
        String::from_utf8_lossy(bytes)
    );
}

#[test]
fn read_model_err() {
    let mut model = KyTea::new();
    assert!(
        model
            .read_model(c"non_existing_model", ModelFormat::Unknown)
            .is_err()
    );

    let mut model = KyTea::new();
    assert!(
        model
            .read_model(c"./Cargo.toml", ModelFormat::Unknown)
            .is_err()
    );

    let mut model = KyTea::new();
    let mut model_stream = IspanStream::new(b"invalid model");
    assert!(
        model
            .read_model_from_stream(&mut model_stream, ModelFormat::Text)
            .is_err()
    );
}

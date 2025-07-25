use serde::de::{Deserialize, SeqAccess, Visitor};
use serde_yaml_bw::Deserializer;
use std::collections::BTreeMap;
use std::fmt;

fn test_error<'de, T>(yaml: &'de str, expected: &str)
where
    T: serde::de::Deserialize<'de> + fmt::Debug,
{
    let result = T::deserialize(Deserializer::from_str(yaml));
    assert_eq!(expected, result.unwrap_err().to_string());

    let mut deserializer = Deserializer::from_str(yaml);
    if let Some(first_document) = deserializer.next() {
        if deserializer.next().is_none() {
            let result = T::deserialize(first_document);
            assert_eq!(expected, result.unwrap_err().to_string());
        }
    }
}

#[cfg(not(miri))]
#[test]
fn test_large_repetition_limit() {
    #[derive(Debug)]
    struct X;

    impl<'de> Visitor<'de> for X {
        type Value = X;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("exponential blowup")
        }

        fn visit_unit<E>(self) -> Result<X, E> {
            Ok(X)
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<X, S::Error>
        where
            S: SeqAccess<'de>,
        {
            while let Some(X) = seq.next_element()? {}
            Ok(X)
        }
    }

    impl<'de> Deserialize<'de> for X {
        fn deserialize<D>(deserializer: D) -> Result<X, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(X)
        }
    }

    use std::fmt::Write;
    let mut yaml = String::new();
    writeln!(&mut yaml, "a0: &a0 ~").unwrap();
    for i in 1..=1000 {
        write!(&mut yaml, "a{}: &a{} [", i, i).unwrap();
        for j in 0..5 {
            if j > 0 {
                yaml.push(',');
            }
            write!(&mut yaml, "*a{}", i - 1).unwrap();
        }
        writeln!(&mut yaml, "]").unwrap();
    }
    writeln!(&mut yaml, "final: *a{}", 1000).unwrap();

    let expected = "repetition limit exceeded";
    test_error::<BTreeMap<String, X>>(&yaml, expected);
}

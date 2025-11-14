use impulse_utils::prelude::{MResult, ServerError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Type {
  Usage(TypeUsage),
  Alias(TypeAlias),
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct TypeUsage {
  pub name: String,
  pub rust_type: String,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct TypeAlias {
  pub alias: String,
  pub rust_type: String,
}

pub const STD_TYPES: [&str; 15] = [
  "bool", "i8", "u8", "f8", "i16", "u16", "i32", "u32", "f32", "i64", "u64", "f64", "String", "Vec", "HashMap",
];

pub fn parse_typedef(typedef_line: &str) -> MResult<Type> {
  assert!(typedef_line.starts_with("type "));

  let parts = typedef_line.split_whitespace().collect::<Vec<_>>();

  if parts.len() < 3 {
    return ServerError::from_public(format!("Invalid type definition: `{typedef_line}`")).bail();
  }
  assert_eq!(parts[0], "type");

  let typename = parse_typename(parts[1]);
  if STD_TYPES.contains(&typename.as_str()) {
    return ServerError::from_public("Do not use standard types! They are imported automatically.").bail();
  }

  let typedesc = parts[2..].join(" ");

  if typedesc.contains("::") {
    Ok(Type::Usage(TypeUsage {
      name: typename,
      rust_type: typedesc,
    }))
  } else {
    Ok(Type::Alias(TypeAlias {
      alias: typename,
      rust_type: typedesc,
    }))
  }
}

pub fn select_typedef(typename: impl AsRef<str>, typedefs: &[Type]) -> HashSet<&Type> {
  typedefs
    .iter()
    .filter_map(|td| match td {
      Type::Usage(usage) if usage.name.as_str().eq(typename.as_ref()) => Some(HashSet::from_iter([td])),
      Type::Alias(alias) if alias.alias.as_str().eq(typename.as_ref()) => match typename_import(&alias.rust_type) {
        Ok(typenames) => Some({
          let mut tns = typenames
            .iter()
            .flat_map(|tn| select_typedef(tn, typedefs))
            .collect::<HashSet<_>>();
          tns.insert(td);
          tns
        }),
        _ => None,
      },
      _ => None,
    })
    .flatten()
    .collect()
}

pub fn convert_typedefs_update(typedefs: &[&Type]) -> Vec<String> {
  let mut typedef_strs = vec![];

  for usage in typedefs.iter().filter_map(|v| match v {
    Type::Usage(usage) => Some(usage),
    _ => None,
  }) {
    if usage.rust_type.split("::").last().unwrap().eq(usage.name.as_str()) {
      typedef_strs.push(format!("use {};", usage.rust_type));
    } else {
      typedef_strs.push(format!("use {} as {};", usage.rust_type, usage.name));
    }
  }

  for alias in typedefs.iter().filter_map(|v| match v {
    Type::Alias(alias) => Some(alias),
    _ => None,
  }) {
    typedef_strs.push(format!("type {} = {};", alias.alias, alias.rust_type));
  }

  typedef_strs
}

pub fn convert_typedefs(typedefs: &[&Type]) -> String {
  let mut typedef_strs = vec![];

  let mut usage_used = false;
  for usage in typedefs.iter().filter_map(|v| match v {
    Type::Usage(usage) => Some(usage),
    _ => None,
  }) {
    if usage.rust_type.split("::").last().unwrap().eq(usage.name.as_str()) {
      typedef_strs.push(format!("use {};", usage.rust_type));
    } else {
      typedef_strs.push(format!("use {} as {};", usage.rust_type, usage.name));
    }
    usage_used = true;
  }

  if usage_used {
    typedef_strs.push(String::new());
  }

  let mut alias_used = false;
  for alias in typedefs.iter().filter_map(|v| match v {
    Type::Alias(alias) => Some(alias),
    _ => None,
  }) {
    typedef_strs.push(format!("type {} = {};", alias.alias, alias.rust_type));
    alias_used = true;
  }

  if alias_used {
    typedef_strs.push(String::new());
  }

  typedef_strs.join("\n")
}

pub fn parse_typename(typename: impl AsRef<str>) -> String {
  if typename.as_ref().eq("str") {
    "String".to_string()
  } else {
    typename.as_ref().to_string()
  }
}

pub fn typename_import(typename: impl AsRef<str>) -> MResult<Vec<String>> {
  let parser = complex_type_enumerator::TypeParser::new()
    .map_err(|e| ServerError::from_private(e).with_public("Can't create complex types parser!"))?;

  Ok(
    parser
      .parse(typename.as_ref())
      .map_err(|e| ServerError::from_private_str(e).with_public("Can't parse complex type!"))?
      .into_iter()
      .filter(|t| !STD_TYPES.contains(&t.as_str()) || t.as_str().eq("HashMap"))
      .collect::<Vec<_>>(),
  )
}

mod complex_type_enumerator {
  use regex::Regex;

  #[derive(Debug, Clone, PartialEq)]
  pub enum TypeNode {
    Simple(String),
    Generic { name: String, params: Vec<TypeNode> },
  }

  impl TypeNode {
    pub fn collect_type_names(&self) -> Vec<String> {
      let mut names = Vec::new();
      self.collect_type_names_recursive(&mut names);
      names
    }

    fn collect_type_names_recursive(&self, names: &mut Vec<String>) {
      match self {
        TypeNode::Simple(name) => {
          names.push(name.clone());
        }
        TypeNode::Generic { name, params } => {
          names.push(name.clone());
          for param in params {
            param.collect_type_names_recursive(names);
          }
        }
      }
    }
  }

  pub struct TypeParser {
    whitespace_re: Regex,
  }

  impl TypeParser {
    pub fn new() -> Result<Self, regex::Error> {
      Ok(Self {
        whitespace_re: Regex::new(r"\s+")?,
      })
    }

    pub fn parse(&self, input: &str) -> Result<Vec<String>, String> {
      let cleaned = self.whitespace_re.replace_all(input.trim(), " ");
      let mut chars = cleaned.chars().peekable();
      let type_node = self.parse_type(&mut chars)?;
      Ok(type_node.collect_type_names())
    }

    fn parse_type(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<TypeNode, String> {
      let name = self.parse_identifier(chars)?;

      // Проверяем, есть ли параметры типа
      if chars.peek() == Some(&'<') {
        chars.next(); // consume '<'
        let params = self.parse_type_params(chars)?;

        // Проверяем закрывающую скобку
        if chars.next() != Some('>') {
          return Err("Expected '>' after type parameters".to_string());
        }

        Ok(TypeNode::Generic { name, params })
      } else {
        Ok(TypeNode::Simple(name))
      }
    }

    fn parse_identifier(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String, String> {
      let mut identifier = String::new();

      // Первый символ должен быть буквой или подчёркиванием
      match chars.peek() {
        Some(c) if c.is_ascii_alphabetic() || *c == '_' => {
          identifier.push(chars.next().unwrap());
        }
        _ => return Err("Expected identifier".to_string()),
      }

      // Последующие символы могут быть буквами, цифрами или подчёркиваниями
      while let Some(&c) = chars.peek() {
        if c.is_ascii_alphanumeric() || c == '_' {
          identifier.push(chars.next().unwrap());
        } else {
          break;
        }
      }

      // Пропускаем пробелы после идентификатора
      self.skip_whitespace(chars);

      Ok(identifier)
    }

    fn parse_type_params(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Vec<TypeNode>, String> {
      let mut params = Vec::new();

      self.skip_whitespace(chars);

      // Если сразу закрывающая скобка, возвращаем пустой список
      if chars.peek() == Some(&'>') {
        return Ok(params);
      }

      loop {
        let param = self.parse_type(chars)?;
        params.push(param);

        self.skip_whitespace(chars);

        match chars.peek() {
          Some(',') => {
            chars.next(); // consume ','
            self.skip_whitespace(chars);
            continue;
          }
          Some('>') => break,
          Some(c) => return Err(format!("Unexpected character '{c}' in type parameters")),
          None => return Err("Unexpected end of input in type parameters".to_string()),
        }
      }

      Ok(params)
    }

    fn skip_whitespace(&self, chars: &mut std::iter::Peekable<std::str::Chars>) {
      while let Some(&c) = chars.peek() {
        if c.is_ascii_whitespace() {
          chars.next();
        } else {
          break;
        }
      }
    }
  }

  // Функция-хелпер для извлечения списка названий типов
  #[allow(dead_code)]
  pub fn extract_type_names(type_str: &str) -> Result<Vec<String>, String> {
    let parser = TypeParser::new().map_err(|e| format!("Failed to create parser: {e}"))?;
    parser.parse(type_str)
  }

  // Функция-хелпер для быстрой проверки наличия HashMap в типе
  #[allow(dead_code)]
  pub fn contains_hashmap(type_str: &str) -> Result<bool, String> {
    let type_names = extract_type_names(type_str)?;
    Ok(type_names.contains(&"HashMap".to_string()))
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    #[test]
    fn test_simple_types() {
      let parser = TypeParser::new().unwrap();

      assert_eq!(parser.parse("String").unwrap(), vec!["String"]);
      assert_eq!(parser.parse("i32").unwrap(), vec!["i32"]);
      assert_eq!(parser.parse("HashMap").unwrap(), vec!["HashMap"]);
    }

    #[test]
    fn test_generic_types() {
      let parser = TypeParser::new().unwrap();

      let result = parser.parse("Vec<String>").unwrap();
      assert_eq!(result, vec!["Vec", "String"]);
    }

    #[test]
    fn test_nested_generic_types() {
      let parser = TypeParser::new().unwrap();

      let result = parser.parse("HashMap<String, Vec<i32>>").unwrap();
      assert_eq!(result, vec!["HashMap", "String", "Vec", "i32"]);
    }

    #[test]
    fn test_hashmap_detection() {
      // Должно определять HashMap
      assert!(contains_hashmap("HashMap").unwrap());
      assert!(contains_hashmap("HashMap<String, i32>").unwrap());
      assert!(contains_hashmap("Vec<HashMap<String, i32>>").unwrap());
      assert!(contains_hashmap("Option<HashMap<String, Vec<i32>>>").unwrap());

      // Не должно ложно срабатывать
      assert!(!contains_hashmap("MyComplexHashMap<String, String>").unwrap());
      assert!(!contains_hashmap("Vec<String>").unwrap());
      assert!(!contains_hashmap("CustomHashMapLike<i32>").unwrap());
    }

    #[test]
    fn test_whitespace_handling() {
      let parser = TypeParser::new().unwrap();

      assert_eq!(
        parser.parse("HashMap< String , i32 >").unwrap(),
        parser.parse("HashMap<String,i32>").unwrap()
      );

      assert_eq!(
        parser.parse("  Vec<  HashMap<String,  i32>  >  ").unwrap(),
        parser.parse("Vec<HashMap<String,i32>>").unwrap()
      );
    }

    #[test]
    fn test_complex_nested_types() {
      let parser = TypeParser::new().unwrap();

      let complex_type = "Result<HashMap<String, Vec<Option<i32>>>, Error>";
      let result = parser.parse(complex_type).unwrap();
      assert_eq!(
        result,
        vec!["Result", "HashMap", "String", "Vec", "Option", "i32", "Error"]
      );
      assert!(contains_hashmap(complex_type).unwrap());

      let no_hashmap_type = "Result<BTreeMap<String, Vec<Option<i32>>>, Error>";
      let result2 = parser.parse(no_hashmap_type).unwrap();
      assert_eq!(
        result2,
        vec!["Result", "BTreeMap", "String", "Vec", "Option", "i32", "Error"]
      );
      assert!(!contains_hashmap(no_hashmap_type).unwrap());
    }

    #[test]
    fn test_helper_function() {
      assert_eq!(
        extract_type_names("HashMap<String, i32>").unwrap(),
        vec!["HashMap", "String", "i32"]
      );
      assert_eq!(
        extract_type_names("Vec<HashMap<K, V>>").unwrap(),
        vec!["Vec", "HashMap", "K", "V"]
      );

      assert!(contains_hashmap("HashMap<String, i32>").unwrap());
      assert!(!contains_hashmap("MyHashMap<String, i32>").unwrap());
      assert!(contains_hashmap("Vec<HashMap<K, V>>").unwrap());
    }

    #[test]
    fn test_error_cases() {
      let parser = TypeParser::new().unwrap();

      assert!(parser.parse("HashMap<String").is_err()); // Незакрытая скобка
      assert_eq!(parser.parse("HashMap<>").unwrap(), vec!["HashMap"]); // Пустые параметры типа
      assert!(parser.parse("HashMap String>").is_err()); // Неправильный синтаксис
      assert!(parser.parse("123Invalid").is_err()); // Некорректный идентификатор
    }
  }
}

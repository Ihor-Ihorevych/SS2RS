use serde_json::{Map, Value};

/// A parser for the RON subset `meta-dump` prints (anonymous structs, lists, strings, numbers,
/// `true`/`false`, `None`), producing the same `serde_json::Value` the JSON output parses to, so
/// that the two formats can be compared.
pub fn parse_ron(text: &str) -> Result<Value, String> {
    let body = text
        .strip_prefix("#![enable(implicit_some)]\n")
        .ok_or("a RON document starts with the implicit_some extension")?;
    let mut p = P {
        s: body.as_bytes(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("trailing text at byte {}", p.i));
    }
    Ok(v)
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at byte {}", c as char, self.i))
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn word(&mut self) -> String {
        let start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphanumeric()
                || matches!(self.s[self.i], b'_' | b'.' | b'-' | b'+'))
        {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).into_owned()
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        let text = std::str::from_utf8(&self.s[self.i..]).map_err(|e| e.to_string())?;
        let mut chars = text.char_indices();
        while let Some((off, c)) = chars.next() {
            match c {
                '"' => {
                    self.i += off + 1;
                    return Ok(out);
                }
                '\\' => match chars.next().ok_or("dangling escape")?.1 {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'u' => {
                        let mut hex = String::new();
                        if chars.next().ok_or("bad \\u")?.1 != '{' {
                            return Err("bad \\u escape".into());
                        }
                        for (_, h) in chars.by_ref() {
                            if h == '}' {
                                break;
                            }
                            hex.push(h);
                        }
                        let v = u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                        out.push(char::from_u32(v).ok_or("bad code point")?);
                    }
                    other => return Err(format!("unknown escape \\{other}")),
                },
                c => out.push(c),
            }
        }
        Err("unterminated string".into())
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek().ok_or("unexpected end")? {
            b'"' => Ok(Value::String(self.string()?)),
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    if self.peek() == Some(b']') {
                        self.i += 1;
                        return Ok(Value::Array(items));
                    }
                    items.push(self.value()?);
                    if self.peek() == Some(b',') {
                        self.i += 1;
                    } else {
                        self.eat(b']')?;
                        return Ok(Value::Array(items));
                    }
                }
            }
            b'(' => {
                self.i += 1;
                let mut map = Map::new();
                loop {
                    if self.peek() == Some(b')') {
                        self.i += 1;
                        return Ok(Value::Object(map));
                    }
                    self.ws();
                    let key = self.word();
                    if key.is_empty() {
                        return Err(format!("expected a field name at byte {}", self.i));
                    }
                    self.eat(b':')?;
                    let v = self.value()?;
                    if map.insert(key.clone(), v).is_some() {
                        return Err(format!("duplicate field {key}"));
                    }
                    if self.peek() == Some(b',') {
                        self.i += 1;
                    } else {
                        self.eat(b')')?;
                        return Ok(Value::Object(map));
                    }
                }
            }
            _ => {
                let w = self.word();
                match w.as_str() {
                    "None" => Ok(Value::Null),
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    "" => Err(format!("unexpected byte at {}", self.i)),
                    n => serde_json::from_str::<Value>(n).map_err(|e| format!("{n}: {e}")),
                }
            }
        }
    }
}

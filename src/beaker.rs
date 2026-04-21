use crate::Substance;

use std::path::{PathBuf, Path, Component};

pub trait Beaker {
    fn query<P: AsRef<Path>>(&self, path: P) -> Result<Substance, PathBuf>;

    ///Writes the substance to the path, Currently the only exception is the key "-" for Sequences effects a push.
    fn insert<P: AsRef<Path>>(&mut self, path: P, value: Substance) -> Result<(), PathBuf>;

    fn delete<P: AsRef<Path>>(&mut self, path: P) -> Result<(), PathBuf> {
        self.insert(path, Substance::Null)
    }
    fn push<P: AsRef<Path>>(&mut self, path: P, value: Substance) -> Result<(), PathBuf> {
        self.insert(path.as_ref().join("-"), value)
    }

    type Copy: Beaker;
    fn copy(&self) -> Self::Copy;
}

fn normalize<P: AsRef<Path>>(path: P) -> Result<Vec<String>, PathBuf> {
    path.as_ref().components().try_fold(vec![], |mut r, c| match c {
        Component::CurDir | Component::RootDir | Component::Prefix(_) => Ok(r),
        Component::ParentDir => r.pop().ok_or(PathBuf::from("../")).map(|_| r),
        Component::Normal(other) => {r.push(other.to_string_lossy().to_string()); Ok(r)},
    })
}

impl Beaker for Substance {
    type Copy = Substance;
    fn copy(&self) -> Self::Copy {self.clone()}

    fn query<P: AsRef<Path>>(&self, path: P) -> Result<Substance, PathBuf> {
        normalize(path).and_then(|p| {
            let mut s = self.clone();
            let mut index = 0;

            loop {
                match p.get(index) {
                    Some(key) => {
                        s = match key.as_str() {
                            "@keys" => s.keys().map(|s| Substance::Seq(s.into_iter().map(Substance::String).collect())),
                            key => {s.remove(key)}
                        }.ok_or(PathBuf::from(p[..1+index].join("/")))?;
                    },
                    None => break Ok(s)
                }
                index += 1;
            }
        })
    }

    fn insert<P: AsRef<Path>>(&mut self, path: P, value: Substance) -> Result<(), PathBuf> {
        normalize(path).and_then(|p| {
            let mut s = self;
            let mut index = 0;

            loop {
                match p.get(index) {
                    Some(last) if p.len()-1 == index => {
                        match s {
                            Self::Map(map) => {map.insert(last.to_string(), value);},
                            Self::Seq(seq) if last == "-" => {seq.push(value);},
                            Self::Seq(seq) if last.parse::<usize>().ok().filter(|l| *l <= seq.len()).is_some() => {
                                seq.insert(last.parse::<usize>().unwrap(), value);
                            },
                            v if matches!(v, Substance::Null) && last == "-" => {
                                *v = Substance::Seq(vec![value]);
                            },
                            _ => Err(PathBuf::from(p[..1+index].join("/")))?
                        }
                        break Ok(());
                    },
                    Some(key) => {
                        s = s.get_mut(key).ok_or(PathBuf::from(p[..1+index].join("/")))?;
                    },
                    None => {
                        *s = value;
                        break Ok(());
                    }
                }
                index += 1;
            }
        })
    }
}

pub struct Offset<B>(B, PathBuf);
impl<B: Beaker> Offset<B> {
    pub fn new(beaker: B, path: PathBuf) -> Self {Offset(beaker, path)}
}
impl<B: Beaker> Beaker for Offset<B> {
    type Copy = Offset<B::Copy>;
    fn copy(&self) -> Self::Copy {Offset::new(self.0.copy(), self.1.clone())}

    fn query<P: AsRef<Path>>(&self, path: P) -> Result<Substance, PathBuf> {
        self.0.query(self.1.join(&path))
    }

    ///Writes the substance to the path, Currently the only exception is the key "-" for Sequences effects a push.
    fn insert<P: AsRef<Path>>(&mut self, path: P, value: Substance) -> Result<(), PathBuf> {
        self.0.insert(self.1.join(&path), value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Primitive;
    use std::collections::BTreeMap;

    #[test]
    fn test() {
        let int = |i| Substance::Field(Primitive::Integer(i));
        let apple = Substance::Seq(vec![int(1), int(2)]);
        let mut value = Substance::Map(BTreeMap::from([
            ("apple".to_string(), apple.clone()),
            ("banana".to_string(), Substance::Map(BTreeMap::from([
                ("single".to_string(), Substance::Seq(vec![int(1)])),
                ("double".to_string(), Substance::Seq(vec![int(1), int(2)])),
            ]))),
        ]));

        assert_eq!(value.query("apple"), Ok(apple.clone()));
        assert_eq!(value.query("banana/double"), Ok(apple));
        assert_eq!(value.query("banana/single/0"), Ok(int(1)));
        assert_eq!(value.query("banana/triple/2"), Err(PathBuf::from("banana/triple")));
        assert_eq!(value.query("banana/single/1"), Err(PathBuf::from("banana/single/1")));
        assert_eq!(value.insert("banana/double/2", int(3)), Ok(()));
        assert_eq!(value.query("banana/double/2"), Ok(int(3)));

        assert_eq!(value.insert("banana/triple/2", int(3)), Err(PathBuf::from("banana/triple")));
        assert_eq!(value.insert("banana/double/4", int(3)), Err(PathBuf::from("banana/double/4")));
    }
}

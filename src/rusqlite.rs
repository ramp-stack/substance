use crate::{Substance, Beaker};

use rusqlite::{OptionalExtension, Connection, Error};

use std::path::{PathBuf, Path};

fn init(connection: &Connection) -> Result<(), Error> {
    connection.execute("CREATE TABLE if not exists Beaker(
        id TEXT NOT NULL PRIMARY KEY,
        value BLOB NOT NULL
    );", [])?;
    Ok(())
}

impl Beaker for Connection {
    type Error = Error;

    fn query<P: AsRef<Path>>(&self, path: P) -> Result<Result<Substance, PathBuf>, Error> {
        init(self)?;
        Ok(self.query_row(
            "SELECT value FROM Beaker WHERE id=0",
            [], |r| Ok(serde_json::from_slice::<Substance>(&r.get::<_, Vec<u8>>(0)?).unwrap()),
        ).optional()?.unwrap_or_default().query(path).unwrap())
    }

    fn insert<P: AsRef<Path>>(&mut self, path: P, value: Substance) -> Result<Result<(), PathBuf>, Error> {
        init(self)?;
        let mut beaker = self.query_row(
            "SELECT value FROM Beaker WHERE id=0",
            [], |r| Ok(serde_json::from_slice::<Substance>(&r.get::<_, Vec<u8>>(0)?).unwrap()),
        ).optional()?.unwrap_or_default();
        let r = beaker.insert(path, value).unwrap();
        self.execute(
            "INSERT INTO Beaker(id, value) VALUES (0, ?1) ON CONFLICT DO UPDATE SET value=excluded.value;",
            [serde_json::to_vec(&beaker).unwrap()],
        )?;
        Ok(r)
    }
}

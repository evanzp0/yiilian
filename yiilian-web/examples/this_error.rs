use thiserror::Error;

use std::error::Error as StdError;

#[derive(Error, Debug)]
enum MyError {

    #[error(transparent)]
    General(#[from] Box<dyn StdError>),
    #[error("IoError: {source}, message: {message}")]
    Io {
        #[source]
        source: Box<dyn StdError>,
        message: String,
    },
    #[error("Other: {0}")]
    Other(std::io::Error) // 省略了 #[from] 
}

fn main() {
    let a = std::fs::rename("from", "to").map_err(MyError::Other);
    match a {
        Ok(_) => todo!(),
        Err(err) => {
            println!("{:?}", err.to_string());
        },
    }
    
    let a = std::fs::rename("from", "to").map_err(|err| MyError::Io { source: err.into(), message: "some error".to_owned() });
    match a {
        Ok(_) => todo!(),
        Err(err) => {
            println!("{:?}", err.to_string());
        },
    }

    let a = std::fs::rename("from", "to").map_err(|err| MyError::General(err.into()));
    match a {
        Ok(_) => todo!(),
        Err(err) => {
            println!("{:?}", err.to_string());
        },
    }
}
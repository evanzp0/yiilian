use std::{error::Error as StdError, fmt::Display};
use std::fmt::Debug;

use futures::Future;

#[tokio::main]
async fn main() {
    let svc = HelloService;

    let svr = Server::new(svc);

    svr.run().await;
}

pub trait Service<Request> {
    type Response;
    type Error;

    fn call(&self, req: Request) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send;
}

#[derive(Clone)]
pub struct HelloService;

impl Service<String> for HelloService
{
    type Response = String;
    type Error = Box<dyn StdError + Send + Sync>;

    async fn call(&self, req: String) -> Result<Self::Response, Self::Error>
    {
        println!("{}", req);
        // Ok(())
        Err(MyError::new("my error").into())
    }
}

pub struct Server<S> {
    service: S
}

impl<S> Server<S> 
where
    S: Service<String> + Clone + Send + 'static,
    S::Response: Debug,
    S::Error: Into<Box<dyn StdError + Send + Sync>> + Debug,
{
    pub fn new(service: S) -> Self {
        Server{
            service
        }
    }

    pub async fn run(&self) {
        let service = self.service.clone();
        tokio::spawn(async move {
            let rst = service.call("abc".to_owned()).await;
            println!("{:?}", rst);
        });
    }
}

#[derive(Debug)]
pub struct MyError {
    msg: String
}

impl MyError {
    pub fn new(msg: &str) -> Self {
        Self {
            msg: msg.to_owned()
        }
    }
}

impl Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl StdError for MyError { }

// pub struct ServiceFn<F> {
//     f: F,
// }

// impl<F, Ret, E> Service<String> for ServiceFn<F>
// where
//     F: Fn() -> Ret,
//     Ret: Future<Output = Result<(), E>>,
//     E: Into<Box<dyn StdError + Send + Sync>>,
// {
//     type Response = String;
//     type Error = E; //Box<dyn StdError + Send + Sync>;

//     async fn call(&self, req: String) -> Result<Self::Response, Self::Error> {
//         todo!()
//     }
// }
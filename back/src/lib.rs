use serde::{Deserialize, Serialize};

pub mod routing;
pub mod streaming;

#[derive(Debug, Clone)]
pub struct AppState{

}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum ServerStatus{
    Alive,
    Unstable,
    Dead,
}
#[derive(Debug, Clone, PartialEq, Serialize,Deserialize)]
enum CoinStatus{
    Tail,
    Head,
}
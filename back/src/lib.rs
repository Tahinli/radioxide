use serde::{Deserialize, Serialize};

pub mod routing;

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
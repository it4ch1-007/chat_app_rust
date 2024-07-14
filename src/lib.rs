use fastrand;
pub static ADJCETIVES:[&str;3] =[
    "mushy",
    "Starry",
    "Amazing",
];
pub fn random_name() -> String{
    let adjective = fastrand::choice(ADJCETIVES).unwrap();
    format!("{adjective}")
}
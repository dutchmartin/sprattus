use {
    std::{
        thread,
        time::Duration,
        result::Result,
        error::Error
    },
};

async fn clap(no: u8) -> () {
    for i in 1..10 {
        thread::sleep(Duration::from_millis(101));
        println!("Person {} clapped for the {}th time", no, i);
    }
    ()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    for i in 0..10 {
        tokio::spawn(async move {
            clap(i).await;
        });
    }

    thread::sleep(Duration::from_millis(3000));
    Ok(())
}
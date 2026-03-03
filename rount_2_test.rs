// round_2
use std::fs::write;
use std::io::{BufRead, Write};
// use std::{arch::aarch64::float32x2_t, env, vec};

// Input: input.txt
// Output: alpha.txt, num.txt, alphanum.txt, float.txt

// abc
// 123
// abc123
// 12.43
//

fn main() {
    let (tx_float, rx_float) = std::sync::mpsc::channel();
    let (tx_num, rx_num) = std::sync::mpsc::channel();
    let (tx_alphanum, rx_alphanum) = std::sync::mpsc::channel();
    let (tx_alpha, rx_alpha) = std::sync::mpsc::channel();

    let j1 = std::thread::spawn(move || {
        consumer_float(rx_float);
    });
    let j2 = std::thread::spawn(move || {
        consumer_num(rx_num);
    });
    let j3 = std::thread::spawn(move || {
        consumer_alphanum(rx_alphanum);
    });
    let j4 = std::thread::spawn(move || {
        consumer_alpha(rx_alpha);
    });

    producer(tx_alpha, tx_num, tx_alphanum, tx_float);

    j1.join().expect("error in joinig hanlde");
    j2.join().expect("error in joinig hanlde");
    j3.join().expect("error in joinig hanlde");
    j4.join().expect("error in joinig hanlde");
}

fn producer(
    tx_alpha: std::sync::mpsc::Sender<String>,
    tx_num: std::sync::mpsc::Sender<i32>,
    tx_alphanum: std::sync::mpsc::Sender<String>,
    tx_float: std::sync::mpsc::Sender<f32>,
) {
    // read a input file using buffered read - as it is a large file
    let input_file =
        std::fs::File::open("/Users/roku/test_prog/input.txt").expect("Failed to open input file");
    let reader = std::io::BufReader::new(input_file);

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        if line.chars().all(|c| c.is_alphabetic()) {
            tx_alpha.send(line).expect("Failed to send alpha data");
        } else if line.chars().all(|c| c.is_numeric()) {
            tx_num
                .send(line.parse::<i32>().expect("Failed to parse num data"))
                .expect("Failed to send num data");
        } else if line.chars().all(|c| c.is_alphanumeric()) {
            tx_alphanum
                .send(line)
                .expect("Failed to send alphanum data");
        } else if line.parse::<f32>().is_ok() {
            tx_float
                .send(line.parse::<f32>().expect("Failed to parse float data"))
                .expect("Failed to send float data");
        } else {
            println!("Line does not match any category: {}", line);
        }
    }

    drop(tx_alpha);
    drop(tx_num);
    drop(tx_alphanum);
    drop(tx_float);
}

fn consumer_float(rx: std::sync::mpsc::Receiver<f32>) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/Users/roku/test_prog/float.txt")
        .expect("Failed to open file");
    loop {
        // let mut count = 0;
        match rx.recv() {
            Ok(float_data) => {
                println!("Received: {}", float_data);
                // count += 1;
                // We need to buffer write it into float.txt
                writeln!(&mut file, "{}", float_data).expect("Failed to write to file");
                // if count == 100 {
                //     file.flush().expect("Failed to flush file");
                //     count = 0;
                // }
            }
            Err(_) => {
                println!("Producer has finished sending data.");
                break;
            }
        }
    }
    file.flush().expect("Failed to flush file");
}

fn consumer_num(rx: std::sync::mpsc::Receiver<i32>) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/Users/roku/test_prog/num.txt")
        .expect("Failed to open file");
    loop {
        match rx.recv() {
            Ok(int_data) => {
                println!("Received: {}", int_data);
                writeln!(&mut file, "{}", int_data).expect("Failed to write to file");
            }
            Err(_) => {
                println!("Producer has finished sending data.");
                break;
            }
        }
    }
    file.flush().expect("Failed to flush file");
}

fn consumer_alphanum(rx: std::sync::mpsc::Receiver<String>) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/Users/roku/test_prog/alphanum.txt")
        .expect("Failed to open file");
    loop {
        match rx.recv() {
            Ok(string_data) => {
                println!("Received: {}", string_data);
                writeln!(&mut file, "{}", string_data).expect("Failed to write to file");
            }
            Err(_) => {
                println!("Producer has finished sending data.");
                break;
            }
        }
    }
    file.flush().expect("Failed to flush file");
}

fn consumer_alpha(rx: std::sync::mpsc::Receiver<String>) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("/Users/roku/test_prog/alpha.txt")
        .expect("Failed to open file");
    loop {
        match rx.recv() {
            Ok(string_data) => {
                println!("Received: {}", string_data);
                writeln!(&mut file, "{}", string_data).expect("Failed to write to file");
            }
            Err(_) => {
                println!("Producer has finished sending data.");
                break;
            }
        }
    }
    file.flush().expect("Failed to flush file");
}


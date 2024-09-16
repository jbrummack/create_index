use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::OnceLock;
use stopwatch::Stopwatch;
use usearch::{new_index, Index, IndexOptions, MetricKind, ScalarKind};
static STORE: OnceLock<Index> = OnceLock::new();
use clap::Parser;

///Converts a .CSV into an uSearch index
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file
    #[arg(short, long)]
    input: String,
    /// Output file
    #[arg(short, long, default_value_t = String::from("index.usearch"))]
    output: String,

    /// Vector length
    #[arg(short, long, default_value_t = 192)]
    vector_length: usize,

    /// Metric kind
    #[arg(short, long, default_value_t = String::from("cos"))]
    metric: String,

    /// Scalar kind
    #[arg(short, long, default_value_t = String::from("f32"))]
    scalar: String,
}

impl Args {
    fn metric_kind(self) -> MetricKind {
        let m = self.metric.to_lowercase();
        if &m == "cos" {
            return MetricKind::Cos;
        }
        MetricKind::IP
    }
    fn scalar_kind(self) -> ScalarKind {
        let m = self.metric.to_lowercase();
        if &m == "f16" || &m == "16" {
            return ScalarKind::F16;
        }
        if &m == "f64" || &m == "64" {
            return ScalarKind::F64;
        }
        if &m == "b1" || &m == "1" {
            return ScalarKind::B1;
        }
        if &m == "i8" || &m == "8" {
            return ScalarKind::I8;
        }
        ScalarKind::F32
    }
}

fn setup_store() -> usize {
    let args = Args::parse();

    println!("{:?}", args);

    let vector_length = args.vector_length;
    let options = IndexOptions {
        dimensions: vector_length,     // necessary for most metric kinds
        metric: MetricKind::Cos,       // or MetricKind::L2sq, MetricKind::Cos ...
        quantization: ScalarKind::F32, // or ScalarKind::F32, ScalarKind::I8, ScalarKind::B1x8 ...
        connectivity: 0,               // zero for auto
        expansion_add: 0,              // zero for auto
        expansion_search: 0,           // zero for auto
        multi: false,                  // Optional: Allow multiple vectors per key, default = False
    };
    //let args: Vec<String> = env::args().collect();
    let file_path = args.input;
    let index: Index = new_index(&options).unwrap();
    println!("Hardware acceleration: {}", index.hardware_acceleration());
    let sw = Stopwatch::start_new();
    let mut file_len: usize = 0;
    if let Ok(file) = read_lines(file_path) {
        file.flatten().for_each(|_| file_len += 1)
    }
    index.reserve(file_len).unwrap();
    println!("File ingestion takes {}ms", sw.elapsed_ms());
    STORE.get_or_init(|| index);
    file_len
}

fn main() -> Result<(), serde_json::Error> {
    let file_len = setup_store();
    let args = Args::parse();
    let file_path = args.input;
    let sw = Stopwatch::start_new();
    if let Ok(lines) = read_lines(file_path) {
        lines.flatten().enumerate().par_bridge().for_each(|(a, b)| {
            //Indexing 7177947 192-vectors takes 9 minutes
            //lines.flatten().enumerate().for_each(|(a, b)| { //Indexing 7177947 192-vectors takes 96 minutes
            let fields: Vec<&str> = b.split(";").collect();
            let vector = fields[4];
            let datas: Result<Vec<f32>, serde_json::Error> = serde_json::from_str(vector);
            if &datas.is_err() == &true {
                println!("timestamp: {} \n{a}:{b} \n{:?}", sw.elapsed_ms(), &datas);
            } else {
                let _ = datas.map(|arr| {
                    if arr.len() == 192 {
                        let _ = STORE.get().map(|ix| ix.add(a as u64, arr.as_slice()));
                    }
                });
            }
        })
    }
    let elapsed = sw.elapsed_ms();
    let projected_time = elapsed / 60_000;
    println!(
        "Indexing {file_len} vectors takes {} minutes ",
        projected_time
    );

    println!(
        "{:?}",
        STORE.get().map(|index| index.save(args.output.as_str()))
    );
    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

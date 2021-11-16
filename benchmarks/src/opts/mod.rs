use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Authenticated Database Benchmark Tool.",
    rename_all = "kebab-case"
)]
pub struct Options {
    #[structopt(short = "a", long)]
    pub algorithm: TestMode,

    #[structopt(short = "k", long, parse(try_from_str = parse_num), default_value="100000")]
    pub total_keys: usize,

    #[structopt(long, default_value = "64")]
    pub seed: u64,

    #[structopt(long, default_value = "150")]
    pub max_time: u64,

    #[structopt(long)]
    pub max_epoch: Option<usize>,

    #[structopt(long, default_value = "50")]
    pub report_epoch: usize,

    #[structopt(long, default_value = "5000")]
    pub profile_epoch: usize,

    #[structopt(long, default_value = "1000")]
    pub epoch_size: usize,

    #[structopt(long = "report-to")]
    pub report_dir: Option<String>,

    #[structopt(long = "db", default_value = "./__benchmarks")]
    pub db_dir: String,
}

impl Options {
    pub fn settings(&self) -> String {
        format!("{:?},{:e}", self.algorithm, self.total_keys)
    }
}

#[derive(Debug, Eq, PartialEq, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum TestMode {
    RAW,
    AMT,
    MPT,
    DMPT,
}

fn parse_num(s: &str) -> Result<usize, String> {
    let base = match s
        .chars()
        .rev()
        .next()
        .ok_or::<String>("empty input".into())?
    {
        'k' | 'K' => 1_000,
        'm' | 'M' => 1_000_000,
        'g' | 'G' => 1_000_000_000,
        _ => 1,
    };
    let num = if base > 1 {
        let mut chars = s.chars();
        chars.next_back();
        chars.as_str()
    } else {
        s
    };
    Ok(base * num.parse::<usize>().map_err(|x| x.to_string())?)
}

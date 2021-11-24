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

    #[structopt(long, default_value = "128")]
    pub cache_size: u64,

    #[structopt(long)]
    pub max_time: Option<u64>,

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

    #[structopt(long, help = "Disable backend stat")]
    pub no_stat: bool,

    #[structopt(long, help = "Enable print root")]
    pub print_root: bool,

    #[structopt(long)]
    pub warmup_to: Option<String>,

    #[structopt(long)]
    pub warmup_from: Option<String>,
}

impl Options {
    pub fn settings(&self) -> String {
        format!("{:?},{:e}", self.algorithm, self.total_keys)
    }
    pub fn warmup_to(&self) -> Option<String> {
        self.warmup_to
            .as_ref()
            .map(|x| format!("{}/{:?}_{:e}/", x, self.algorithm, self.total_keys))
    }
    pub fn warmup_from(&self) -> Option<String> {
        self.warmup_from
            .as_ref()
            .map(|x| format!("{}/{:?}_{:e}/", x, self.algorithm, self.total_keys))
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

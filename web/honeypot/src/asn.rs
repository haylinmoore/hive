use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct As {
    pub number: u32,
    pub country: String,
    pub name: String,
}

struct Range {
    start: u128,
    end: u128,
    owner: usize,
}

#[derive(Default)]
pub struct Table {
    ranges: Vec<Range>,
    owners: Vec<As>,
}

fn key(ip: IpAddr) -> u128 {
    match ip {
        IpAddr::V4(a) => u128::from(a.to_ipv6_mapped()),
        IpAddr::V6(a) => u128::from(a),
    }
}

impl Table {
    pub fn parse(reader: impl BufRead) -> Table {
        let mut t = Table::default();
        let mut index = std::collections::HashMap::new();
        for line in reader.lines().map_while(Result::ok) {
            let f: Vec<&str> = line.split('\t').collect();
            let [start, end, number, country, name] = f[..] else {
                continue;
            };
            let (Ok(start), Ok(end), Ok(number)) =
                (start.parse(), end.parse(), number.parse::<u32>())
            else {
                continue;
            };
            if number == 0 {
                continue;
            }
            let owner = *index.entry(number).or_insert_with(|| {
                t.owners.push(As {
                    number,
                    country: country.to_string(),
                    name: name.to_string(),
                });
                t.owners.len() - 1
            });
            t.ranges.push(Range {
                start: key(start),
                end: key(end),
                owner,
            });
        }
        t.ranges.sort_by_key(|r| r.start);
        t
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<&As> {
        let k = key(ip.to_canonical());
        let i = self
            .ranges
            .partition_point(|r| r.start <= k)
            .checked_sub(1)?;
        let r = &self.ranges[i];
        (k <= r.end).then(|| &self.owners[r.owner])
    }
}

#[derive(Default)]
pub struct Cache(Mutex<Option<(SystemTime, Arc<Table>)>>);

impl Cache {
    pub fn get(&self, path: &Path) -> Arc<Table> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let mut slot = self.0.lock().unwrap();
        if let (Some(m), Some((at, table))) = (modified, slot.as_ref())
            && *at == m
        {
            return table.clone();
        }
        let Some(m) = modified else {
            return Arc::default();
        };
        let table = std::fs::File::open(path)
            .map(|f| Table::parse(BufReader::new(flate2::read::GzDecoder::new(f))))
            .unwrap_or_default();
        let table = Arc::new(table);
        *slot = Some((m, table.clone()));
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_v4_and_v6_ranges() {
        let data = "1.0.0.0\t1.0.0.255\t13335\tUS\tCLOUDFLARENET\n\
                    1.0.1.0\t1.0.3.255\t0\tNone\tNot routed\n\
                    2001:db8::\t2001:db8::ffff\t64500\tZZ\tEXAMPLE\n";
        let t = Table::parse(data.as_bytes());
        assert_eq!(t.lookup("1.0.0.7".parse().unwrap()).unwrap().number, 13335);
        assert!(t.lookup("1.0.2.1".parse().unwrap()).is_none());
        assert_eq!(
            t.lookup("2001:db8::1".parse().unwrap()).unwrap().name,
            "EXAMPLE"
        );
        assert!(t.lookup("9.9.9.9".parse().unwrap()).is_none());
    }
}

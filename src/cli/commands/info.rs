use crate::db::queries::{find_planet_for_info, get_aliases};
use crate::normalize::normalize_text;
use crate::ui::info;
use anyhow::Result;
use rusqlite::Connection;

const LABEL_W: usize = 24;

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "-".into())
}

fn opt_str(v: Option<&str>) -> &str {
    v.unwrap_or("-")
}

pub fn run(con: &Connection, planet: String) -> Result<()> {
    let pn = normalize_text(&planet);
    let p = match find_planet_for_info(con, &pn)? {
        Some(p) => p,
        None => anyhow::bail!("No planet found matching '{}'", planet),
    };

    let aliases = get_aliases(con, p.fid)?;

    info("Planet Information");
    println!();

    println!("{:<LABEL_W$}: {}", "FID", p.fid);
    println!("{:<LABEL_W$}: {}", "Planet", p.planet);
    println!("{:<LABEL_W$}: {}", "planet_norm", p.planet_norm);

    println!("{:<LABEL_W$}: {}", "Region", opt_str(p.region.as_deref()));
    println!("{:<LABEL_W$}: {}", "Sector", opt_str(p.sector.as_deref()));
    println!("{:<LABEL_W$}: {}", "System", opt_str(p.system.as_deref()));
    println!("{:<LABEL_W$}: {}", "Grid", opt_str(p.grid.as_deref()));

    println!("{:<LABEL_W$}: {}", "X (parsecs)", p.x);
    println!("{:<LABEL_W$}: {}", "Y (parsecs)", p.y);

    println!("{:<LABEL_W$}: {}", "Canon", opt(p.canon));
    println!("{:<LABEL_W$}: {}", "Legends", opt(p.legends));
    println!("{:<LABEL_W$}: {}", "zm", opt(p.zm));
    println!("{:<LABEL_W$}: {}", "Latitude", opt(p.lat));
    println!("{:<LABEL_W$}: {}", "Longitude", opt(p.long));

    println!("{:<LABEL_W$}: {}", "Status", opt_str(p.status.as_deref()));
    println!(
        "{:<LABEL_W$}: {}",
        "Reference",
        opt_str(p.reference.as_deref())
    );
    println!(
        "{:<LABEL_W$}: {}",
        "Canonical Region",
        opt_str(p.c_region.as_deref())
    );
    println!(
        "{:<LABEL_W$}: {}",
        "Canonical Region (long)",
        opt_str(p.c_region_li.as_deref())
    );

    let label_w_new = LABEL_W - 3;
    println!();
    println!("Name aliases:");
    println!(
        "{:>2} {:<label_w_new$}: {}",
        "-",
        "name0",
        opt_str(p.name0.as_deref())
    );
    println!(
        "{:>2} {:<label_w_new$}: {}",
        "-",
        "name1",
        opt_str(p.name1.as_deref())
    );
    println!(
        "{:>2} {:<label_w_new$}: {}",
        "-",
        "name2",
        opt_str(p.name2.as_deref())
    );

    println!();
    if aliases.is_empty() {
        println!("Aliases: -");
    } else {
        println!("Aliases:");
        for a in aliases {
            let src = a.source.as_deref().unwrap_or("unknown");
            println!("  - {:<label_w_new$} ({})", a.alias, src);
        }
    }

    println!();
    println!("{:<LABEL_W$}: {}", "Info URL", p.info_planet_url());

    Ok(())
}

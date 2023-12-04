use std::fs::File;
use clap::Parser;

use pdb::{AddressMap, FallibleIterator, ItemFinder, PDB, PdbInternalSectionOffset, SymbolTable, TypeIndex, TypeInformation};

struct DumpContext<'a, 'b> {
    pdb: &'a mut PDB<'b, File>,
    symbol_table: &'a SymbolTable<'b>,
    address_map: &'a AddressMap<'b>,
    tpi_finder: &'a ItemFinder<'a, TypeIndex>,
    opt: &'a Args,
}

impl<'a, 'b> DumpContext<'a, 'b> {
    fn print_row(&self, offset: PdbInternalSectionOffset, kind: &str, name: pdb::RawString<'_>) {
        println!(
            "{:x}\t{:x}\t{}\t{}",
            offset.section, offset.offset, kind, name
        );
    }

    fn print_symbol(&self, symbol: &pdb::Symbol<'_>) -> pdb::Result<()> {
        match symbol.parse()? {
            // pdb::SymbolData::Public(data) => {
            //     self.print_row(data.offset, "function", data.name);
            //     if let Some(rva) = data.offset.to_rva(&self.address_map) {
            //         println!("\t\tRVA:{}", rva);
            //     }
            // }
            // pdb::SymbolData::Procedure(data) => {
            //     self.print_row(data.offset, "function", data.name);
            //     if let Some(rva) = data.offset.to_rva(&self.address_map) {
            //         println!("\t\tRVA:{}", rva);
            //     }
            // }
            pdb::SymbolData::Data(data) => {
                if self.opt.variables {
                    self.print_row(data.offset, "data", data.name);

                    if let Some(rva) = data.offset.to_rva(&self.address_map) {
                        println!("\t\tRVA:{}", rva);
                    }
                }
            }
            _ => {
                // ignore everything else
            }
        }

        Ok(())
    }

    fn walk_symbols(&self, mut symbols: pdb::SymbolIter<'_>) -> pdb::Result<()> {
        println!("segment\toffset\tkind\tname");

        while let Some(symbol) = symbols.next()? {
            match self.print_symbol(&symbol) {
                Ok(_) => (),
                Err(e) => ()
            }
        }

        Ok(())
    }

    fn dump_pdb(opt: &Args) -> pdb::Result<()> {
        let file = std::fs::File::open(&opt.filename)?;
        let mut pdb = pdb::PDB::open(file)?;
        let symbol_table = pdb.global_symbols()?;
        let address_map = pdb.address_map()?;

        let dbi = pdb.debug_information()?;
        let mut modules = dbi.modules()?;

        let tpi = pdb.type_information()?;
        let mut tpi_finder = tpi.finder();
        let mut tpii = tpi.iter();

        if opt.types {
            println!("Types:")
        }
        while let Some(tp) = tpii.next()? {
            tpi_finder.update(&tpii);
            if opt.types {
                let typ = tp.parse()?;
                print!("#0x{:x}: ", tp.index().0);
                if let Some(name) = typ.name() {
                    println!("{:?}", name)
                } else {
                    println!("{:?}", typ)
                }
            }
        }

        let ctx = DumpContext {
            pdb: &mut pdb,
            symbol_table: &symbol_table,
            address_map: &address_map,
            tpi_finder: &tpi_finder,
            opt,
        };


        if opt.variables {
            println!("Global symbols:");
            ctx.walk_symbols(ctx.symbol_table.iter())?;
        }

        if opt.modules {
            println!("Module private symbols:");
            while let Some(module) = modules.next()? {
                println!("Module: {}", module.object_file_name());
                let info = match ctx.pdb.module_info(&module)? {
                    Some(info) => info,
                    None => {
                        println!("  no module info");
                        continue;
                    }
                };
                ctx.walk_symbols(info.symbols()?)?;
            }
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    types: bool,
    #[arg(short, long)]
    variables: bool,
    #[arg(short, long)]
    modules: bool,
    filename: String,
}

fn main() {
    let args = Args::parse();
    match DumpContext::dump_pdb(&args) {
        Ok(_) => (),
        Err(e) => eprintln!("error dumping PDB: {}", e),
    }
}

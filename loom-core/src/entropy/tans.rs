pub struct TansSymbol {
    pub symbol: u8,
    pub freq: u16,
    pub start: u16,
}

pub struct TansTable {
    pub table_size: usize,
    pub state_table: Vec<u16>,
    pub symbol_table: Vec<u8>,
}

impl TansTable {
    pub fn new(symbols: &[TansSymbol], bits: u8) -> Self {
        let table_size = 1 << bits;
        let mut state_table = vec![0u16; table_size];
        let mut symbol_table = vec![0u8; table_size];

        let mut next_state = vec![0u16; 256];
        for sym in symbols {
            next_state[sym.symbol as usize] = sym.freq;
        }

        let mut pos = 0;
        for sym in symbols {
            for _ in 0..sym.freq {
                if pos < table_size {
                    symbol_table[pos] = sym.symbol;
                    state_table[pos] = next_state[sym.symbol as usize];
                    next_state[sym.symbol as usize] += 1;
                    pos += 1;
                }
            }
        }

        Self {
            table_size,
            state_table,
            symbol_table,
        }
    }

    pub fn decode_step(&self, state: u32) -> (u8, u32) {
        let idx = (state as usize) % self.table_size;
        let sym = self.symbol_table[idx];
        let next_st = (state >> 4) | (self.state_table[idx] as u32);
        (sym, next_st)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tans_table_creation() {
        let symbols = vec![
            TansSymbol {
                symbol: 0,
                freq: 8,
                start: 0,
            },
            TansSymbol {
                symbol: 1,
                freq: 8,
                start: 8,
            },
        ];
        let table = TansTable::new(&symbols, 4);
        assert_eq!(table.table_size, 16);
        let (sym, _next) = table.decode_step(5);
        assert!(sym <= 1);
    }
}

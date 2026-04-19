use anyhow::Result;
use rustfst::algorithms::compose;
use rustfst::prelude::*;

static G2P_MODEL_FILE: &[u8] = include_bytes!("../../../data/g2p.fst");

pub struct G2pModel {
    fst: VectorFst<TropicalWeight>,
}

impl G2pModel {
    pub fn new() -> Result<Self> {
        let fst = VectorFst::<TropicalWeight>::load(G2P_MODEL_FILE)?;
        Ok(Self { fst })
    }

    pub fn phoneticize(&self, text: &str) -> Result<Vec<String>> {
        let isyms = self.fst.input_symbols().unwrap();
        let mut inputs: Vec<Label> = Vec::new();
        for ch in text.chars() {
            if let Some(seq) = isyms.get_label(ch.to_string()) {
                inputs.push(seq);
            }
        }

        // input graph
        let mut input_fst: VectorFst<TropicalWeight> = VectorFst::new();
        let mut state = input_fst.add_state();
        input_fst.set_start(state).unwrap();

        for sym in inputs {
            let next_state = input_fst.add_state();
            input_fst
                .add_tr(state, Tr::new(sym, sym, TropicalWeight::one(), next_state))
                .unwrap();
            state = next_state;
        }

        input_fst.set_final(state, TropicalWeight::one()).unwrap();

        let composed_fst: VectorFst<TropicalWeight> =
            compose::compose::<_, _, VectorFst<TropicalWeight>, _, _, _>(input_fst, &self.fst)
                .unwrap();

        let shortest_fst: VectorFst<_> = shortest_path(&composed_fst).unwrap();
        let osyms = shortest_fst.output_symbols().unwrap();

        let mut phonemes = vec![];
        for path in shortest_fst.paths_iter() {
            for label in path.olabels {
                if label == 2 {
                    continue; // Skip epsilon or some specific marker?
                }
                if let Some(symbol) = osyms.get_symbol(label) {
                    phonemes.push(symbol.replace("|", ""));
                }
            }
        }

        Ok(phonemes)
    }
}

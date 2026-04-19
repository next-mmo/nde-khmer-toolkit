//! Grapheme-to-phoneme conversion via Phonetisaurus-style FST composition.
//!
//! Ported from Python `sosap.Model` / `sosap-rs/main.rs`.

use anyhow::{anyhow, Result};
use rustfst::algorithms::{compose, shortest_path};
use rustfst::prelude::*;
use std::path::Path;

/// A loaded G2P FST model. Wraps a `VectorFst<TropicalWeight>`.
pub struct G2PModel {
    fst: VectorFst<TropicalWeight>,
}

impl G2PModel {
    /// Load a Phonetisaurus FST file from disk.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let fst = VectorFst::<TropicalWeight>::read(path.as_ref())?;
        Ok(Self { fst })
    }

    /// Load a Phonetisaurus FST from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let fst = VectorFst::<TropicalWeight>::load(bytes)?;
        Ok(Self { fst })
    }

    /// Convert Khmer text into its phonemic representation.
    ///
    /// Returns a single string concatenating the output symbols of the shortest
    /// path through the composed FST, stripping `|` delimiters.
    pub fn phoneticize(&self, text: &str) -> Result<String> {
        let isyms = self
            .fst
            .input_symbols()
            .ok_or_else(|| anyhow!("input symbol table missing"))?;

        let mut inputs: Vec<Label> = Vec::with_capacity(text.chars().count());
        for ch in text.chars() {
            let sym = ch.to_string();
            let label = isyms
                .get_label(&sym)
                .ok_or_else(|| anyhow!("unknown input symbol: {:?}", sym))?;
            inputs.push(label);
        }

        let mut input_fst: VectorFst<TropicalWeight> = VectorFst::new();
        let mut state = input_fst.add_state();
        input_fst.set_start(state)?;
        for sym in inputs {
            let next_state = input_fst.add_state();
            input_fst.add_tr(
                state,
                Tr::new(sym, sym, TropicalWeight::one(), next_state),
            )?;
            state = next_state;
        }
        input_fst.set_final(state, TropicalWeight::one())?;

        let composed_fst: VectorFst<TropicalWeight> =
            compose::compose::<_, _, VectorFst<TropicalWeight>, _, _, _>(input_fst, &self.fst)?;

        let shortest_fst: VectorFst<TropicalWeight> = shortest_path(&composed_fst)?;
        let osyms = shortest_fst
            .output_symbols()
            .ok_or_else(|| anyhow!("output symbol table missing"))?;

        let mut phonemes = Vec::new();
        for path in shortest_fst.paths_iter() {
            for label in path.olabels {
                if label == 2 {
                    continue;
                }
                if let Some(symbol) = osyms.get_symbol(label) {
                    phonemes.push(symbol.replace('|', ""));
                }
            }
        }

        Ok(phonemes.join(""))
    }
}

use rosu_pp::Beatmap;
use std::error::Error;

pub struct PPCalculator {
    beatmap_path: String,
}

impl PPCalculator {
    pub fn new(beatmap_path: String) -> Self {
        Self { beatmap_path }
    }

    pub fn calculate_pp(&self, beatmap_id: u32, mods: u32, combo: u32, accuracy: f64, misses: u32) -> Result<(f64, f64, f64), Box<dyn Error>> {
        let map = Beatmap::from_path(&self.beatmap_path)?;

        let diff_attrs = rosu_pp::Difficulty::new()
            .mods(mods)
            .calculate(&map);

        let stars = diff_attrs.stars();

        let perf_attrs = rosu_pp::Performance::new(diff_attrs)
            .mods(mods)
            .combo(combo)
            .accuracy(accuracy)
            .misses(misses)
            .calculate();

        let pp = perf_attrs.pp();

        let max_pp = perf_attrs.performance()
            .mods(mods)
            .calculate()
            .pp();

        Ok((stars, pp, max_pp))
    }

    pub fn calculate_beatmap_details(&self, mods: u32) -> Result<(f64, f64, f64, f64, f64, f64, f64), Box<dyn Error>> {
        let map = Beatmap::from_path(&self.beatmap_path)?;

        let diff_attrs = rosu_pp::Difficulty::new()
            .mods(mods)
            .calculate(&map);

        let stars = diff_attrs.stars();

        let perf_attrs = rosu_pp::Performance::new(diff_attrs)
            .mods(mods);

        let max_pp = perf_attrs.clone().accuracy(100.0).calculate().pp();

        let pp_95_fc = perf_attrs.clone().accuracy(95.0).calculate().pp();

        let pp_96_fc = perf_attrs.clone().accuracy(96.0).calculate().pp();

        let pp_97_fc = perf_attrs.clone().accuracy(97.0).calculate().pp();

        let pp_98_fc = perf_attrs.clone().accuracy(98.0).calculate().pp();

        let pp_99_fc = perf_attrs.clone().accuracy(99.0).calculate().pp();
            
        Ok((stars,max_pp, pp_95_fc, pp_96_fc, pp_97_fc, pp_98_fc, pp_99_fc))
    }
}

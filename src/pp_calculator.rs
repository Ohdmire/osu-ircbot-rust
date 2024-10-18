use rosu_pp::Beatmap;
use std::error::Error;

pub struct PPCalculator {
    beatmap_path: String,
}

impl PPCalculator {
    pub fn new(beatmap_path: String) -> Self {
        Self { beatmap_path }
    }

    pub fn calculate_pp(&self, mods: u32, combo: u32, accuracy: f64, misses: u32) -> Result<(f64, f64, f64), Box<dyn Error>> {
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

    pub fn calculate_beatmap_details(&self, mods: u32) -> Result<(f64, f64, f64, f64, f64, f64, f64, f64), Box<dyn Error>> {
        let map = Beatmap::from_path(&self.beatmap_path)?;

        let diff_attrs = rosu_pp::Difficulty::new()
            .mods(mods)
            .calculate(&map);

        let stars = diff_attrs.stars();

        let perf_attrs = rosu_pp::Performance::new(diff_attrs)
            .mods(mods)
            .calculate();

        let pp = perf_attrs.pp();

        let max_pp = perf_attrs.clone().performance()
            .mods(mods)
            .calculate()
            .pp();

        let pp_95_fc = perf_attrs.clone().performance()
            .mods(mods)
            .accuracy(0.95)
            .calculate()
            .pp();

        let pp_96_fc = perf_attrs.clone().performance()
            .mods(mods)
            .accuracy(0.96)
            .calculate()
            .pp();

        let pp_97_fc = perf_attrs.clone().performance()
            .mods(mods)
            .accuracy(0.97)
            .calculate()
            .pp();

        let pp_98_fc = perf_attrs.clone().performance()
            .mods(mods)
            .accuracy(0.98)
            .calculate()
            .pp();

        let pp_99_fc = perf_attrs.clone().performance()
            .mods(mods)
            .accuracy(0.99)
            .calculate()
            .pp();
            
        Ok((stars, pp, max_pp, pp_95_fc, pp_96_fc, pp_97_fc, pp_98_fc, pp_99_fc))
    }
}

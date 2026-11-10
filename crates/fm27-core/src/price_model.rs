//! Fair-price model for the save's own transfer market.
//!
//! A log-linear hedonic model: expected market value from what a player is
//! (age, CA/PA, position, club level, contract) and what he does (quality
//! percentile, minutes). Five-fold out-of-fold predictions keep a player's own
//! fee out of his price, so "value gap" is not self-fitted. Players without a
//! usable fee get the average of the five fold models.
//!
//! The model is deliberately small and dependency-free: a ridge regression on
//! standardized features solved with Gaussian elimination.

use chrono::NaiveDate;

pub const PRICE_MODEL: &str = "hedonic-ridge-v2";

const FOLDS: usize = 5;
const MIN_TRAINING_ROWS: usize = 200;
const MIN_FOLD_ROWS: usize = 10;
const MIN_CV_R2: f64 = 0.05;
const RIDGE_LAMBDA: f64 = 1.0;

/// Intercept + quality and quality squared, CA and CA squared, PA, age and age
/// squared, minutes, reputation, reputation known, contract years, contract
/// known, league known, and eight position dummies.
pub const FEATURE_COUNT: usize = 22;

/// One scored player handed to the price model.
#[derive(Debug, Clone)]
pub struct PriceInput {
    pub uid: u32,
    /// Known fee above zero. Free transfers and unknown fees stay `None`.
    pub fee: Option<u64>,
    pub features: Vec<f64>,
}

/// Fitted model. `folds` stores one ridge per held-out fold.
#[derive(Debug, Clone)]
pub struct PriceModel {
    folds: Vec<Ridge>,
    residual_sd: f64,
    cv_r2: f64,
    training_rows: usize,
}

impl PriceModel {
    /// Fit only when the market is large enough to price it out of sample.
    pub fn fit(inputs: &[PriceInput]) -> Option<Self> {
        let training: Vec<usize> = inputs
            .iter()
            .enumerate()
            .filter_map(|(index, input)| {
                let fee = input.fee?;
                (fee > 0
                    && input.features.len() == FEATURE_COUNT
                    && input.features.iter().all(|value| value.is_finite()))
                .then_some(index)
            })
            .collect();
        if training.len() < MIN_TRAINING_ROWS {
            return None;
        }

        let mut folds = Vec::with_capacity(FOLDS);
        let mut oof: Vec<Option<f64>> = vec![None; inputs.len()];
        for fold in 0..FOLDS {
            let mut rows: Vec<&[f64]> = Vec::new();
            let mut targets: Vec<f64> = Vec::new();
            let mut holdout: Vec<usize> = Vec::new();
            for &index in &training {
                if fold_of(inputs[index].uid, FOLDS) == fold {
                    holdout.push(index);
                    continue;
                }
                rows.push(inputs[index].features.as_slice());
                targets.push(log_fee(inputs[index].fee.unwrap_or(0)));
            }
            if rows.len() < MIN_FOLD_ROWS || holdout.is_empty() {
                return None;
            }
            let ridge = Ridge::fit(&rows, &targets, RIDGE_LAMBDA)?;
            for index in holdout {
                oof[index] = Some(ridge.predict(&inputs[index].features));
            }
            folds.push(ridge);
        }

        let mean = training
            .iter()
            .map(|index| log_fee(inputs[*index].fee.unwrap_or(0)))
            .sum::<f64>()
            / training.len() as f64;
        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;
        for &index in &training {
            let actual = log_fee(inputs[index].fee.unwrap_or(0));
            let predicted = oof[index]?;
            ss_res += (actual - predicted).powi(2);
            ss_tot += (actual - mean).powi(2);
        }
        if ss_tot <= 0.0 {
            return None;
        }
        let cv_r2 = 1.0 - ss_res / ss_tot;
        if !cv_r2.is_finite() || cv_r2 < MIN_CV_R2 {
            return None;
        }
        let residual_sd = (ss_res / (training.len().saturating_sub(1).max(1)) as f64).sqrt();
        Some(Self {
            folds,
            residual_sd,
            cv_r2,
            training_rows: training.len(),
        })
    }

    /// Price for a player the model was not trained on: average the folds in
    /// log space, which is a geometric mean of the five prices.
    pub fn predict_market(&self, features: &[f64]) -> f64 {
        if self.folds.is_empty() {
            return 0.0;
        }
        let log = self
            .folds
            .iter()
            .map(|fold| fold.predict(features))
            .sum::<f64>()
            / self.folds.len() as f64;
        price_from_log(log)
    }

    /// Price from the fold that excluded this player. Used for training rows so
    /// the gap cannot be fitted on the fee it is compared with.
    pub fn predict_oof(&self, uid: u32, features: &[f64]) -> f64 {
        match self.folds.get(fold_of(uid, self.folds.len())) {
            Some(fold) => price_from_log(fold.predict(features)),
            None => 0.0,
        }
    }

    /// Out-of-fold R² on log fees; exposed for diagnostics and tests.
    pub fn cv_r2(&self) -> f64 {
        self.cv_r2
    }

    /// Out-of-fold residual standard deviation on log fees.
    pub fn residual_sd(&self) -> f64 {
        self.residual_sd
    }

    pub fn training_rows(&self) -> usize {
        self.training_rows
    }
}

/// Remaining contract length in years at the save's game date.
pub fn contract_years(contract_end: Option<&str>, game_date: Option<&str>) -> Option<f64> {
    let end = NaiveDate::parse_from_str(contract_end?, "%Y-%m-%d").ok()?;
    let clock = NaiveDate::parse_from_str(game_date?, "%Y-%m-%d").ok()?;
    Some((end - clock).num_days() as f64 / 365.25)
}

fn log_fee(fee: u64) -> f64 {
    (fee as f64 + 1.0).ln()
}

/// Turn a log-space prediction into a price. The model estimates the conditional
/// median: with large fee dispersion (σ ≈ 1 on real saves) the log-normal mean
/// correction would lift every price by ~70% and label half the market a
/// bargain, so the median is the honest comparison point.
fn price_from_log(log: f64) -> f64 {
    log.exp_m1().max(0.0)
}

/// Knuth multiplicative hash, so uids created in club order still mix across
/// folds.
fn fold_of(uid: u32, folds: usize) -> usize {
    let mixed = uid.wrapping_mul(2_654_435_761) >> 16;
    mixed as usize % folds.max(1)
}

#[derive(Debug, Clone)]
struct Ridge {
    means: Vec<f64>,
    sds: Vec<f64>,
    beta: Vec<f64>,
}

impl Ridge {
    fn fit(rows: &[&[f64]], targets: &[f64], lambda: f64) -> Option<Self> {
        let width = rows.first()?.len();
        if width == 0 || rows.len() < width + 1 || targets.len() != rows.len() {
            return None;
        }
        if rows.iter().any(|row| row.len() != width) {
            return None;
        }
        let count = rows.len() as f64;
        let mut means = vec![0.0; width];
        for index in 1..width {
            means[index] = rows.iter().map(|row| row[index]).sum::<f64>() / count;
        }
        let mut sds = vec![1.0; width];
        for index in 1..width {
            let variance = rows
                .iter()
                .map(|row| (row[index] - means[index]).powi(2))
                .sum::<f64>()
                / count;
            sds[index] = variance.sqrt().max(1e-9);
        }

        let mut xtx = vec![vec![0.0; width]; width];
        let mut xty = vec![0.0; width];
        for (row, target) in rows.iter().zip(targets) {
            let mut standardized = vec![1.0; width];
            for index in 1..width {
                standardized[index] = (row[index] - means[index]) / sds[index];
            }
            for left in 0..width {
                xty[left] += standardized[left] * target;
                for right in 0..width {
                    xtx[left][right] += standardized[left] * standardized[right];
                }
            }
        }
        // The intercept is not penalized; every other coefficient is.
        for (index, row) in xtx.iter_mut().enumerate().take(width).skip(1) {
            row[index] += lambda;
        }
        let beta = solve(xtx, xty)?;
        Some(Self { means, sds, beta })
    }

    fn predict(&self, features: &[f64]) -> f64 {
        if features.len() != self.beta.len() {
            return 0.0;
        }
        self.beta[0]
            + features
                .iter()
                .enumerate()
                .skip(1)
                .map(|(index, value)| {
                    ((value - self.means[index]) / self.sds[index]) * self.beta[index]
                })
                .sum::<f64>()
    }
}

fn solve(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let size = rhs.len();
    for column in 0..size {
        let pivot = (column..size).max_by(|left, right| {
            matrix[*left][column]
                .abs()
                .partial_cmp(&matrix[*right][column].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let diagonal = matrix[column][column];
        if diagonal.abs() < 1e-12 {
            continue;
        }
        for row in (column + 1)..size {
            let factor = matrix[row][column] / diagonal;
            if factor == 0.0 {
                continue;
            }
            let (head, tail) = matrix.split_at_mut(row);
            let source = &head[column];
            let target = &mut tail[0];
            for (target_value, source_value) in target.iter_mut().zip(source).skip(column) {
                *target_value -= factor * source_value;
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let diagonal = matrix[row][row];
        if diagonal.abs() < 1e-12 {
            solution[row] = 0.0;
            continue;
        }
        let mut total = rhs[row];
        for index in (row + 1)..size {
            total -= matrix[row][index] * solution[index];
        }
        solution[row] = total / diagonal;
    }
    Some(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(uid: u32, fee: Option<u64>, quality: f64) -> PriceInput {
        // Keep every other feature constant so the tests isolate the target.
        let mut features = vec![1.0; FEATURE_COUNT];
        features[1] = quality;
        PriceInput { uid, fee, features }
    }

    #[test]
    fn contract_years_reads_dates_and_ignores_missing_values() {
        let years = contract_years(Some("2029-06-30"), Some("2026-07-01")).expect("years");
        assert!((years - 3.0).abs() < 0.05);
        assert_eq!(contract_years(None, Some("2026-07-01")), None);
        assert_eq!(contract_years(Some("2029-06-30"), None), None);
        assert_eq!(contract_years(Some("not a date"), Some("2026-07-01")), None);
    }

    #[test]
    fn refuses_to_price_a_market_that_is_too_small() {
        let inputs: Vec<PriceInput> = (0..50)
            .map(|index| input(index, Some(1_000 + index as u64), 0.5))
            .collect();
        assert!(PriceModel::fit(&inputs).is_none());
    }

    #[test]
    fn fits_a_linear_log_price_surface_out_of_sample() {
        let inputs: Vec<PriceInput> = (0..400)
            .map(|index| {
                let quality = (index % 20) as f64 / 20.0;
                // Deterministic wobble keeps the fit from being degenerate.
                let wobble = ((index * 37) % 7) as f64 / 100.0;
                let log_price = 12.0 + 2.0 * quality + wobble;
                PriceInput {
                    uid: index as u32 + 1,
                    fee: Some(log_price.exp_m1().round() as u64),
                    features: input(0, None, quality).features,
                }
            })
            .collect();
        let model = PriceModel::fit(&inputs).expect("model");
        assert!(model.cv_r2() > 0.8, "cv_r2={}", model.cv_r2());
        assert_eq!(model.training_rows(), 400);
        assert!(model.residual_sd() > 0.0);

        let cheap = input(1, None, 0.1).features;
        let expensive = input(2, None, 0.9).features;
        assert!(model.predict_market(&expensive) > model.predict_market(&cheap));
    }

    #[test]
    fn out_of_fold_prediction_ignores_the_players_own_fee() {
        let base: Vec<PriceInput> = (0..300)
            .map(|index| {
                let quality = (index % 10) as f64 / 10.0;
                let fee = (1_000.0 * (2.0 * quality).exp() + (index % 7) as f64 * 50.0) as u64;
                input(index as u32 + 1, Some(fee), quality)
            })
            .collect();
        let model = PriceModel::fit(&base).expect("model");
        let target = &base[7];
        let fold = model
            .folds
            .get(fold_of(target.uid, model.folds.len()))
            .expect("fold");
        let first = fold.predict(&target.features);

        // Change only the target's fee; its own fold model never saw the target,
        // so the fold's log prediction must not move. (The public price also
        // applies a market-wide residual correction, which may shift slightly.)
        let mut changed = base.clone();
        changed[7].fee = Some(target.fee.unwrap_or(0) * 50);
        let other = PriceModel::fit(&changed).expect("model");
        let other_fold = other
            .folds
            .get(fold_of(target.uid, other.folds.len()))
            .expect("fold");
        let second = other_fold.predict(&target.features);
        assert!((first - second).abs() < 1e-9, "{first} vs {second}");
    }

    #[test]
    fn free_and_unknown_fees_use_the_fold_average() {
        let inputs: Vec<PriceInput> = (0..300)
            .map(|index| {
                let quality = (index % 10) as f64 / 10.0;
                let fee = (1_000.0 * (2.0 * quality).exp() + (index % 7) as f64 * 50.0) as u64;
                input(index as u32 + 1, Some(fee), quality)
            })
            .collect();
        let model = PriceModel::fit(&inputs).expect("model");
        let unknown = input(9_999, None, 0.5);
        let market = model.predict_market(&unknown.features);
        let fold_prices: Vec<f64> = model
            .folds
            .iter()
            .map(|fold| price_from_log(fold.predict(&unknown.features)))
            .collect();
        let low = fold_prices.iter().copied().fold(f64::INFINITY, f64::min);
        let high = fold_prices.iter().copied().fold(0.0, f64::max);
        assert!(
            market >= low && market <= high,
            "{market} not in {low}..{high}"
        );
    }

    #[test]
    fn feature_count_is_stable() {
        let features = input(1, Some(1), 0.5).features;
        assert_eq!(features.len(), FEATURE_COUNT);
    }
}

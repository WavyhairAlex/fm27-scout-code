import type { Attributes, CaEstimate, CaEstimateRequest, Positions } from "./types";

/**
 * Exact FM recommended-CA (RCA) estimator.
 *
 * Port of the community reverse-engineering of the game's CA calculation
 * (https://www.playgm.cc/thread-986285-1-1.html). It follows the same weighted
 * average, distribution correction, position-combination cost and familiarity
 * scaling as `fm27-core::ability`. The desktop app uses the Rust implementation;
 * this module keeps the browser mock and tests in sync.
 */

const POSITION_SLOTS = [
  "gk",
  "sw",
  "dl",
  "dc",
  "dr",
  "dm",
  "ml",
  "mc",
  "mr",
  "aml",
  "amc",
  "amr",
  "stc",
  "wbl",
  "wbr",
] as const satisfies readonly (keyof Positions)[];

const POSITION_NAMES = [
  "GK",
  "SW",
  "DL",
  "DC",
  "DR",
  "DM",
  "ML",
  "MC",
  "MR",
  "AML",
  "AMC",
  "AMR",
  "STC",
  "WBL",
  "WBR",
] as const;

const FOOT_WEIGHTS = [3, 4, 4.5, 4, 5, 5, 6, 5.5, 7, 7.5];

const ATTRIBUTE_WEIGHTS: Record<string, readonly number[]> = {
  corners: [0, 1, 1, 1, 1, 1, 1, 1, 1, 1],
  crossing: [0, 2, 1, 3, 1, 5, 1, 5, 1, 2],
  dribbling: [0, 1, 1, 2, 2, 3, 2, 5, 3, 5],
  finishing: [0, 1, 1, 1, 2, 2, 2, 2, 3, 8],
  first_touch: [1, 3, 2, 3, 4, 4, 6, 5, 5, 6],
  free_kick_taking: [0, 1, 1, 1, 1, 1, 1, 1, 1, 1],
  heading: [1, 2, 5, 1, 1, 1, 1, 1, 1, 6],
  long_shots: [0, 1, 1, 1, 3, 2, 3, 2, 3, 2],
  long_throws: [0, 1, 1, 1, 1, 1, 1, 1, 1, 1],
  marking: [0, 3, 8, 2, 3, 1, 3, 1, 1, 1],
  passing: [3, 2, 2, 3, 4, 3, 6, 2, 4, 2],
  penalty_taking: [0, 1, 1, 1, 1, 1, 1, 1, 1, 1],
  tackling: [0, 4, 5, 3, 7, 2, 3, 2, 2, 1],
  technique: [1, 2, 1, 3, 3, 4, 4, 4, 5, 4],
  anticipation: [3, 3, 5, 3, 5, 3, 3, 3, 3, 5],
  bravery: [6, 2, 2, 1, 1, 1, 1, 1, 1, 1],
  composure: [2, 2, 2, 2, 2, 3, 3, 3, 3, 6],
  concentration: [6, 4, 4, 3, 3, 2, 2, 2, 2, 2],
  decisions: [10, 7, 10, 5, 8, 5, 7, 5, 6, 5],
  leadership: [2, 1, 2, 1, 1, 1, 1, 1, 1, 1],
  off_the_ball: [0, 1, 1, 2, 1, 2, 3, 2, 3, 6],
  positioning: [5, 4, 8, 3, 5, 1, 3, 1, 2, 2],
  teamwork: [2, 2, 1, 2, 2, 2, 2, 2, 2, 1],
  vision: [1, 2, 1, 2, 4, 3, 6, 3, 6, 2],
  work_rate: [1, 2, 2, 2, 4, 3, 3, 3, 3, 2],
  acceleration: [6, 7, 6, 8, 6, 8, 6, 10, 9, 10],
  agility: [8, 6, 6, 5, 6, 6, 6, 6, 6, 6],
  balance: [2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
  jumping_reach: [1, 2, 6, 1, 1, 1, 1, 1, 1, 5],
  pace: [3, 5, 5, 6, 4, 6, 5, 10, 7, 7],
  stamina: [1, 6, 3, 7, 4, 5, 6, 7, 6, 6],
  strength: [4, 4, 6, 4, 5, 3, 4, 3, 3, 6],
  aerial_reach: [6, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  command_of_area: [6, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  communication: [5, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  handling: [8, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  kicking: [5, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  one_on_ones: [4, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  reflexes: [8, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  throwing: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
};

interface SlotInfo {
  column: number;
  layer: number;
  route: number | null;
  region: number;
}

const SLOT_INFO: readonly SlotInfo[] = [
  { column: 0, layer: 0, route: null, region: 0 }, // GK
  { column: 2, layer: 1, route: 2, region: 1 }, // SW
  { column: 1, layer: 1, route: 0, region: 1 }, // DL
  { column: 2, layer: 1, route: 2, region: 1 }, // DC
  { column: 1, layer: 1, route: 1, region: 1 }, // DR
  { column: 4, layer: 2, route: 2, region: 2 }, // DM
  { column: 5, layer: 3, route: 0, region: 2 }, // ML
  { column: 6, layer: 3, route: 2, region: 2 }, // MC
  { column: 5, layer: 3, route: 1, region: 2 }, // MR
  { column: 7, layer: 4, route: 0, region: 2 }, // AML
  { column: 8, layer: 4, route: 2, region: 2 }, // AMC
  { column: 7, layer: 4, route: 1, region: 2 }, // AMR
  { column: 9, layer: 5, route: 2, region: 3 }, // STC
  { column: 3, layer: 2, route: 0, region: 2 }, // WBL
  { column: 3, layer: 2, route: 1, region: 2 }, // WBR
];

const POSITION_ALIASES: Record<string, number> = {
  GK: 0,
  SW: 1,
  DL: 2,
  DRL: 2,
  DC: 3,
  DR: 4,
  DM: 5,
  ML: 6,
  MRL: 6,
  MC: 7,
  MR: 8,
  AML: 9,
  AMRL: 9,
  AMC: 10,
  AMR: 11,
  STC: 12,
  ST: 12,
  SC: 12,
  WBL: 13,
  WBRL: 13,
  WBR: 14,
};

interface ColumnResult {
  candidate: number;
  initialCa: number;
  weightedAverage: number;
  excessAverage: number;
  totalWeight: number;
  iterations: number;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

function positionSlot(position: string): number | null {
  return POSITION_ALIASES[position.trim().toUpperCase()] ?? null;
}

function retention(familiarity: number): number | null {
  if (familiarity >= 18) return 1;
  switch (familiarity) {
    case 17:
      return 0.99;
    case 16:
      return 0.98;
    case 15:
      return 0.96;
    case 14:
      return 0.94;
    case 13:
      return 0.92;
    case 12:
      return 0.88;
    case 11:
      return 0.84;
    case 10:
      return 0.8;
    case 9:
      return 0.72;
    case 8:
      return 0.64;
    case 7:
      return 0.56;
    case 6:
      return 0.4;
    case 5:
      return 0.24;
    case 4:
      return 0.08;
    case 2:
    case 3:
      return 0.01;
    default:
      return null;
  }
}

function positionCost(positions: Partial<Positions>): number {
  let familiaritySum = 0;
  const layers = [false, false, false, false, false, false];
  const routes = [false, false, false];
  const regions = [false, false, false, false];

  SLOT_INFO.forEach((info, slot) => {
    const familiarity = clamp(positions[POSITION_SLOTS[slot]] ?? 0, 0, 20);
    familiaritySum += Math.max(familiarity - 10, 0);
    if (familiarity < 15) return;
    layers[info.layer] = true;
    if (info.route !== null) routes[info.route] = true;
    regions[info.region] = true;
  });

  const layerCount = layers.filter(Boolean).length;
  const routeCount = routes.filter(Boolean).length;
  const middleLayers = Number(layers[2]) + Number(layers[3]) + Number(layers[4]);
  const midfieldPenalty = middleLayers >= 2 ? 3 : 0;
  const sidePenalty = routes[0] || routes[1] ? 3 : 0;

  let regionCount = regions.filter(Boolean).length;
  if (regions[0] && regionCount > 1) regionCount += 2;
  if (regions[1] && regions[3]) regionCount = Math.max(regionCount, 3);
  if (layers[2] && layers[4]) regionCount = Math.max(regionCount, 2);
  const regionCost = regionCount < 2 ? 0 : regionCount * regionCount * (regionCount + 1);

  const value = familiaritySum + 3 * (layerCount + routeCount) - midfieldPenalty - sidePenalty + regionCost;
  return clamp(value, 40, 140) - 40;
}

function countedFeet(leftFoot: number, rightFoot: number): number[] {
  if (rightFoot >= 100) return [leftFoot];
  if (leftFoot >= 100) return [rightFoot];
  return [leftFoot, rightFoot];
}

function estimateColumn(
  attributes: Attributes,
  column: number,
  leftFoot: number,
  rightFoot: number,
  cost: number,
): ColumnResult {
  const left = clamp(leftFoot, 0, 100);
  const right = clamp(rightFoot, 0, 100);
  const feet = countedFeet(left, right);
  const footWeight = FOOT_WEIGHTS[column];

  let weightedSum = 0;
  let totalWeight = 0;
  let nonzeroCount = 0;
  for (const [key, weights] of Object.entries(ATTRIBUTE_WEIGHTS)) {
    const weight = weights[column];
    if (weight <= 0) continue;
    weightedSum += clamp(attributes[key] ?? 1, 1, 100) * weight;
    totalWeight += weight;
    nonzeroCount += 1;
  }
  for (const foot of feet) {
    weightedSum += foot * footWeight;
    totalWeight += footWeight;
    nonzeroCount += 1;
  }

  const weightedAverage = weightedSum / totalWeight;
  const initialCa = Math.max(1, Math.floor((weightedAverage / 5 - 6) * 20));
  const threshold = (3.75 * totalWeight) / nonzeroCount;

  let candidate = initialCa;
  let previousT: number | null = null;
  let iterations = 0;
  let lastExcess = 0;

  for (let round = 1; round <= 100; round += 1) {
    const deduction = Math.min(Math.floor(cost / 3), Math.floor(Math.max(candidate, 0) / 4));
    const base = ((candidate - deduction) / 20 + 6) * 5;

    let excess = 0;
    for (const [key, weights] of Object.entries(ATTRIBUTE_WEIGHTS)) {
      const weight = weights[column];
      if (weight <= 0) continue;
      const deviation = ((clamp(attributes[key] ?? 1, 1, 100) - base) * weight) / 5;
      if (deviation > threshold) excess += deviation - threshold;
      else if (deviation < -threshold) excess += deviation + threshold;
    }
    for (const foot of feet) {
      const deviation = ((foot - base) * footWeight) / 5;
      if (deviation > threshold) excess += deviation - threshold;
      else if (deviation < -threshold) excess += deviation + threshold;
    }
    lastExcess = excess;

    const correctedAverage = (weightedSum + excess * 3.75) / totalWeight;
    const error = base - correctedAverage;
    let t: number;
    if (error >= 0) {
      t = Math.floor(error + 0.5);
      if (t === 0) t = 1;
    } else {
      t = Math.trunc(error - 0.5);
    }

    const limit = Math.max(1, 5 - Math.floor(round / 10));
    t = clamp(t, -limit, limit);

    if (t !== 0 && previousT !== null) {
      if (Math.abs(t - previousT) >= 6 && Math.abs(t) >= 2) {
        t = Math.trunc(t / 2);
      }
      if (Math.abs(t) > Math.abs(previousT)) {
        t = Math.sign(t) * Math.abs(previousT);
      }
      if (t === -previousT && Math.abs(t) > 1) {
        t -= Math.sign(t);
      }
      if (previousT > 0 && t > previousT) {
        candidate -= 1;
        iterations += 1;
        if (candidate <= 0) candidate = initialCa;
        continue;
      }
      if (previousT < 0) {
        t = Math.max(t, previousT);
      }
    }

    previousT = t;
    iterations += 1;
    if (t > 0) candidate -= 1;
    else if (t < 0) candidate += 1;
    else break;
    if (candidate <= 0) candidate = initialCa;
  }

  return {
    candidate: Math.max(1, candidate),
    initialCa: Math.max(1, initialCa),
    weightedAverage,
    excessAverage: lastExcess / totalWeight,
    totalWeight,
    iterations,
  };
}

export function estimateCaExact(request: CaEstimateRequest): CaEstimate | null {
  const cost = positionCost(request.positions);
  const requestedPosition =
    request.position && request.position.toUpperCase() !== "AUTO" ? request.position : null;
  const requestedSlot = requestedPosition ? positionSlot(requestedPosition) : null;
  if (requestedPosition && requestedSlot === null) return null;

  const slots = requestedSlot === null ? SLOT_INFO.map((_, slot) => slot) : [requestedSlot];
  let best: CaEstimate | null = null;

  for (const slot of slots) {
    const familiarity = clamp(request.positions[POSITION_SLOTS[slot]] ?? 0, 0, 20);
    if (familiarity < 2) continue;
    const retentionValue = retention(familiarity);
    if (retentionValue === null) continue;
    const result = estimateColumn(
      request.attributes,
      SLOT_INFO[slot].column,
      request.left_foot,
      request.right_foot,
      cost,
    );
    const estimated = clamp(Math.floor(result.candidate * retentionValue + 0.5), 1, 200);
    const candidate: CaEstimate = {
      position: POSITION_NAMES[slot],
      estimated_ca: estimated,
      weighted_average: result.weightedAverage,
      excess_average: result.excessAverage,
      total_weight: result.totalWeight,
      position_cost: cost,
      initial_ca: result.initialCa,
      iterations: result.iterations,
      left_foot: clamp(request.left_foot, 0, 100),
      right_foot: clamp(request.right_foot, 0, 100),
    };
    if (!best || estimated > best.estimated_ca) best = candidate;
  }

  return best;
}

import { describe, expect, it } from "vitest";
import { commandMetrics, recordCommandDuration, resetPerfMetrics } from "./perf";

describe("command metrics", () => {
  it("reports p50, p95 and the maximum per command", () => {
    resetPerfMetrics();
    for (let value = 1; value <= 100; value += 1) {
      recordCommandDuration("query_players", value);
    }
    recordCommandDuration("export_players", 250);

    const metrics = commandMetrics();
    expect(metrics.map((metric) => metric.command)).toEqual([
      "export_players",
      "query_players",
    ]);

    const query = metrics.find((metric) => metric.command === "query_players");
    expect(query).toMatchObject({
      count: 100,
      p50_ms: 50,
      p95_ms: 95,
      max_ms: 100,
    });

    const exported = metrics.find((metric) => metric.command === "export_players");
    expect(exported).toMatchObject({
      count: 1,
      p50_ms: 250,
      p95_ms: 250,
      max_ms: 250,
    });
  });

  it("keeps a bounded number of samples", () => {
    resetPerfMetrics();
    for (let value = 1; value <= 600; value += 1) {
      recordCommandDuration("query_players", value);
    }
    const metric = commandMetrics()[0];
    expect(metric.count).toBe(512);
    expect(metric.max_ms).toBe(600);
  });
});

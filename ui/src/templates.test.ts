import { describe, expect, it } from "vitest";
import { templateSession } from "./templates";

describe("guided templates", () => {
  it("builds stopped inspectable graphs with bounded matrices", () => {
    const template = templateSession("gaming-discord");
    expect(template.name).toBe("Gaming + Discord");
    expect(template.revision).toBe(7);
    expect(template.edges.map((edge) => [edge.sourceNode, edge.destinationNode])).toEqual([["mic", "voice"], ["voice", "headphones"]]);
    expect(template.edges.map((edge) => edge.matrix)).toEqual([[1], [1, 1]]);
  });

  it("returns independent template snapshots and distinct names", () => {
    const first = templateSession("processed-microphone");
    first.nodes[1].parameters.gainDb = 12;
    const second = templateSession("processed-microphone");
    expect(second.nodes[1].parameters.gainDb).toBe(-3);
    expect(templateSession("mix-minus").name).toBe("Mix-minus conversation");
  });
});

import { describe, expect, it } from "vitest";
import { partialStringField, streamedContent } from "../src/lib/partialJson";

describe("partial JSON field extraction", () => {
  it("reads a field once its key and colon arrive", () => {
    expect(partialStringField('{"path":"a.txt","content":"hel', "content")).toBe("hel");
  });

  it("handles escapes split across deltas", () => {
    expect(partialStringField('{"content":"line1\\', "content")).toBe("line1");
    expect(partialStringField('{"content":"line1\\nline2"}', "content")).toBe("line1\nline2");
    expect(partialStringField('{"content":"a\\tb\\"c"}', "content")).toBe('a\tb"c');
  });

  it("handles \\u escapes and waits for the full sequence", () => {
    expect(partialStringField('{"content":"ok\\u00', "content")).toBe("ok");
    expect(partialStringField('{"content":"ok\\u00e1"}', "content")).toBe("oká");
  });

  it("returns null when the key is not there yet", () => {
    expect(partialStringField('{"path":"a.tx', "content")).toBeNull();
    expect(partialStringField("", "content")).toBeNull();
  });

  it("picks path for write and newText for edit", () => {
    expect(streamedContent("write", '{"content":"abc"}')).toBe("abc");
    expect(streamedContent("edit", '{"path":"x","edits":[{"newText":"novo"}],"oldText":"velho"}')).toBe("novo");
    expect(streamedContent("bash", '{"command":"ls"}')).toBeNull();
  });
});

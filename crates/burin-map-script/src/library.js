/**
 * The `burin` object a document script sees.
 *
 * A document is a run of calls: `burin.batch({…})`, `burin.water(…)`,
 * `burin.shore(…)`, in the order they are drawn, ended by `burin.draw()`.
 * Each call builds one entry and keeps it; nothing is returned to hold. The
 * same script runs at the CLI, in the browser, and from Python and Node,
 * because each of those is just a front end that runs it and reads the atlas.
 *
 *   burin.batch({ shade: burin.lerp("#e8dcc8", "#2a1810", burin.tone) })
 *   burin.water("dots", { color: "#9db4c0" })
 *   burin.area({ at: "37.75,-122.45", size_km: [8, 8] })
 *   burin.draw()
 *
 * `where` keeps a fill to a band of heights, which is how a contour belt is
 * written — a loop, the point of a real language:
 *
 *   for (const [from, to] of [[0, 200], [200, 400]])
 *     burin.fill({ color: "#8a7a5c", where: { elevMin: from, elevMax: to } })
 *
 * Nothing joins the document until `burin.draw()`; a call made before then is
 * just collected, so the order of the calls is the draw order.
 */
(function () {
  "use strict";

  const B = globalThis.__burin;
  delete globalThis.__burin;

  /** The entry list being built, in draw order. Reset for each run. */
  const items = [];

  /** An object, or a helpful error: the argument is usually the mistake. */
  function object(v, what) {
    if (v === undefined || v === null) return {};
    if (typeof v !== "object" || Array.isArray(v)) {
      throw new Error(`${what} must be an object`);
    }
    return v;
  }

  /** A number, or an error naming the key. */
  function number(v, what) {
    if (typeof v !== "number" || !isFinite(v)) {
      throw new Error(`${what} must be a number`);
    }
    return v;
  }

  /** A colour: a string burin reads, or a tree. */
  function color(v, what) {
    if (typeof v === "string") return v;
    if (Array.isArray(v)) return v;
    throw new Error(`${what} must be a colour string or a tree`);
  }

  /** A scalar: a number, or a tree. */
  function scalar(v, what) {
    if (typeof v === "number" && isFinite(v)) return v;
    if (Array.isArray(v)) return v;
    throw new Error(`${what} must be a number or a tree`);
  }

  // ---- expression trees ---------------------------------------------------
  // A tree is written the way the engine saves one: `["lerp", a, b, t]`. The
  // helpers exist so a document does not have to spell that out.

  function tree(op, args) {
    return [op, ...args];
  }

  const burin = {
    /**
     * A batch of hatching, stippling or a fill. Everything burin's `batch`
     * takes: `name`, `angle`, `color`, `shade`, `width`, `cap`, `ground`,
     * `draw`, `where`, and `style` or the parameters by name.
     */
    batch(options) {
      const o = object(options, "burin.batch");
      const { style, where, color: c, shade, width, draw, ground, name, angle, cap, ...rest } = o;
      if (draw !== undefined && draw !== "lines" && draw !== "dots" && draw !== "fill") {
        throw new Error(`burin.batch draw must be "lines", "dots" or "fill", not ${JSON.stringify(draw)}`);
      }
      if (ground !== undefined && ground !== "land" && ground !== "sea") {
        throw new Error(`burin.batch ground must be "land" or "sea", not ${JSON.stringify(ground)}`);
      }
      const out = {};
      if (name !== undefined) out.name = String(name);
      if (angle !== undefined) out.angle = number(angle, "burin.batch angle");
      if (cap !== undefined) out.cap = String(cap);
      if (ground !== undefined) out.ground = ground;
      if (draw !== undefined) out.draw = draw;
      if (c !== undefined) out.color = color(c, "burin.batch color");
      if (shade !== undefined) out.shade = color(shade, "burin.batch shade");
      if (width !== undefined) out.width = scalar(width, "burin.batch width");
      if (where !== undefined) out.where = band(where, "burin.batch where");
      out.style = { ...styleParams(style), ...styleParams(rest) };
      return push(B.batch(JSON.stringify(out)));
    },

    /** A fill of a band: what a contour belt is. */
    fill(options) {
      return burin.batch({ ...object(options, "burin.fill"), draw: "fill" });
    },

    /** Lines on water: `burin.water("lines" | "ripples" | "dots", {…})`. */
    water(kind, options) {
      if (kind !== undefined && kind !== null && typeof kind === "object") {
        options = kind;
        kind = "lines";
      }
      const k = kind ?? "lines";
      if (k !== "lines" && k !== "ripples" && k !== "dots") {
        throw new Error(`burin.water draws lines, ripples or dots, not ${JSON.stringify(k)}`);
      }
      const o = object(options, "burin.water");
      const { style, where, color: c, shade, width, follow, angle, name, cap, ...rest } = o;
      const out = { draw: k, kind: "water" };
      if (name !== undefined) out.name = String(name);
      if (angle !== undefined) out.angle = number(angle, "burin.water angle");
      if (cap !== undefined) out.cap = String(cap);
      if (follow !== undefined) out.follow = String(follow);
      if (c !== undefined) out.color = color(c, "burin.water color");
      if (shade !== undefined) out.shade = color(shade, "burin.water shade");
      if (width !== undefined) out.width = scalar(width, "burin.water width");
      if (where !== undefined) out.where = band(where, "burin.water where", true);
      out.style = { ...styleParams(style), ...styleParams(rest) };
      return push(B.water(JSON.stringify(out)));
    },

    /** The shore, the zero contour; or its own style. */
    shore(options) {
      const o = object(options, "burin.shore");
      const { style, color: c, shade, width, ...rest } = o;
      const out = { kind: "shore" };
      if (c !== undefined) out.color = color(c, "burin.shore color");
      if (shade !== undefined) out.shade = color(shade, "burin.shore shade");
      if (width !== undefined) out.width = scalar(width, "burin.shore width");
      out.style = { ...styleParams(style), ...styleParams(rest) };
      return push(B.shore(JSON.stringify(out)));
    },

    /**
     * Where to draw. The same keys as the document's area block: `place`,
     * `at`, `bounds`, `size_km`, `size_mm`, `scale`, `pen_mm`. A front end
     * reads it back; the command line only fills in what its own flags left
     * unset.
     */
    area(options) {
      const o = object(options, "burin.area");
      return B.area(JSON.stringify(o));
    },

    /** Draw what has been built: the document, for the engine. */
    draw() {
      if (items.length === 0) {
        throw new Error("the document has nothing to draw: add a batch with burin.batch({…})");
      }
      return B.draw(JSON.stringify(items));
    },

    /** The document as JSON, without drawing it: for saving or inspecting. */
    json() {
      return B.atlas(JSON.stringify(items));
    },

    /** The style parameters, by name, with their help: what an editor lists. */
    parameters() {
      return JSON.parse(B.parameters());
    },

    /** The fields a tree may read at a point. */
    elev: "elev",
    tone: "tone",
    dist: "dist",
    depth: "depth",

    /** A colour between two others, by `t`. */
    lerp(a, b, t) {
      return tree("lerp", [color(a, "burin.lerp"), color(b, "burin.lerp"), t]);
    },
    /** An rgb colour from three trees. */
    rgb(r, g, b) {
      return tree("rgb", [r, g, b]);
    },
    /** `x` kept between `lo` and `hi`. */
    clamp(x, lo, hi) {
      return tree("clamp", [x, lo, hi]);
    },
    /** A smooth step between two edges. */
    smoothstep(edge0, edge1, x) {
      return tree("smoothstep", [edge0, edge1, x]);
    },
    /** 0 below `edge`, 1 above. */
    step(edge, x) {
      return tree("step", [edge, x]);
    },
    abs(x) {
      return tree("abs", [x]);
    },
    floor(x) {
      return tree("floor", [x]);
    },
    ceil(x) {
      return tree("ceil", [x]);
    },
    round(x) {
      return tree("round", [x]);
    },
    min(a, b) {
      return tree("min", [a, b]);
    },
    max(a, b) {
      return tree("max", [a, b]);
    },
    /** `t` as a plain fraction, so a colour can be made from it. */
    mix(a, b, t) {
      return tree("mix", [color(a, "burin.mix"), color(b, "burin.mix"), number(t, "burin.mix t")]);
    },
  };

  /** Keep one entry, and hand it back so a call can be used as a value. */
  function push(entry) {
    items.push(entry);
    return entry;
  }

  /** A batch's style parameters: numbers only, by their short names too. */
  function styleParams(v) {
    const o = object(v, "style");
    const out = {};
    for (const [k, val] of Object.entries(o)) {
      if (val === undefined || val === null) continue;
      out[k] = number(val, `style ${k}`);
    }
    return out;
  }

  /** The heights or depths a fill keeps to, in metres. */
  function band(v, what, water) {
    const o = object(v, what);
    const keys = water
      ? ["depthMin", "depthMax", "depthFade"]
      : ["elevMin", "elevMax", "depthMin", "depthMax"];
    const out = {};
    for (const k of keys) {
      if (o[k] !== undefined) out[k] = number(o[k], `${what}.${k}`);
    }
    if (out.elevMin !== undefined && out.elevMax !== undefined && out.elevMin > out.elevMax) {
      throw new Error(`${what}: elevMin is above elevMax`);
    }
    if (out.depthMin !== undefined && out.depthMax !== undefined && out.depthMin > out.depthMax) {
      throw new Error(`${what}: depthMin is above depthMax`);
    }
    return out;
  }

  globalThis.burin = Object.freeze(burin);
})();

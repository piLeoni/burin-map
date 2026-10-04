/**
 * The `burin` object a document script sees.
 *
 * A document is a list of batches and water passes, in the order they are
 * drawn; this builds that list and hands it back as the atlas the engine
 * reads. The front ends are the same one burin-map-core exposes to draw.land,
 * so a style written here means the same thing there.
 *
 *   const hills = burin.batch({ shade: burin.lerp("#e8dcc8", "#2a1810", burin.tone) })
 *   const sea = burin.water("dots", { color: "#9db4c0" })
 *   burin.atlas().batch(hills).water(sea).draw()
 *
 * `where` keeps a fill to a band of heights, which is how a contour belt is
 * written:
 *
 *   for (const [from, to] of [[0, 200], [200, 400]])
 *     burin.atlas().batch(burin.fill({ color: "#8a7a5c", where: { elevMin: from, elevMax: to } }))
 */
(function () {
  "use strict";

  const B = globalThis.__burin;
  delete globalThis.__burin;

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

  const FIELDS = ["elev", "tone", "dist", "depth"];

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
      return B.batch(JSON.stringify(out));
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
      return B.water(JSON.stringify(out));
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
      return B.shore(JSON.stringify(out));
    },

    /** The style parameters, by name, with their help: what an editor lists. */
    parameters() {
      return JSON.parse(B.parameters());
    },

    /** The document being built, in draw order. */
    atlas() {
      return new Atlas();
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

  /** The document under construction, in draw order. */
  function Atlas() {
    this.items = [];
  }
  Atlas.prototype.batch = function (b) {
    if (b === undefined) throw new Error("atlas.batch needs a batch: burin.batch({…})");
    this.items.push(b);
    return this;
  };
  Atlas.prototype.water = function (w) {
    if (w === undefined) throw new Error('atlas.water needs a pass: burin.water("dots", {…})');
    this.items.push(w);
    return this;
  };
  Atlas.prototype.shore = function (s) {
    this.items.push(s === undefined ? burin.shore() : s);
    return this;
  };
  /** The document, for the engine: an atlas JSON string. */
  Atlas.prototype.draw = function () {
    if (this.items.length === 0) {
      throw new Error("the document has nothing to draw: add a batch with burin.batch({…})");
    }
    return B.draw(JSON.stringify(this.items));
  };
  /** The same, without drawing: the atlas as JSON, for saving or inspecting. */
  Atlas.prototype.json = function () {
    return B.atlas(JSON.stringify(this.items));
  };

  globalThis.burin = Object.freeze(burin);
})();

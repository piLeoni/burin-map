//! Python bindings: the `burin_map._burin` extension module. The public Python
//! API lives in `python/burin_map/__init__.py`; this is the thin native layer
//! under it. Arrays cross as raw little-endian bytes so the wheel needs no
//! numpy at build time and one abi3 wheel serves every Python version.

use burin_map::{Error, Style};
use burin_map_core::{Batch, Line};
use pyo3::exceptions::{PyConnectionError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

fn py_err(e: Error) -> PyErr {
    match e {
        Error::Input(m) => PyValueError::new_err(m),
        Error::Net(m) => PyConnectionError::new_err(m),
        Error::Data(m) => PyRuntimeError::new_err(m),
    }
}

fn style_from(d: Option<&Bound<'_, PyDict>>) -> PyResult<Style> {
    let mut st = Style::default();
    if let Some(d) = d {
        for (k, v) in d.iter() {
            let name: String = k.extract()?;
            if !st.set(&name, v.extract::<f64>()?) {
                return Err(PyValueError::new_err(format!(
                    "unknown style parameter {name:?}"
                )));
            }
        }
    }
    Ok(st)
}

/// Flatten polylines to (points as f32 x,y pairs, u32 end offsets in points).
fn pack<'py>(py: Python<'py>, lines: &[Line]) -> (Bound<'py, PyBytes>, Bound<'py, PyBytes>) {
    let mut pts = Vec::with_capacity(lines.iter().map(|l| l.len() * 8).sum());
    let mut ends = Vec::with_capacity(lines.len() * 4);
    let mut n = 0u32;
    for l in lines {
        for p in l {
            pts.extend_from_slice(&p[0].to_le_bytes());
            pts.extend_from_slice(&p[1].to_le_bytes());
        }
        n += l.len() as u32;
        ends.extend_from_slice(&n.to_le_bytes());
    }
    (PyBytes::new(py, &pts), PyBytes::new(py, &ends))
}

fn f32s<'py>(py: Python<'py>, v: &[f32]) -> Bound<'py, PyBytes> {
    let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    PyBytes::new(py, &b)
}

/// A finished drawing.
#[pyclass(frozen, module = "burin_map._burin")]
struct Map {
    inner: burin_map::Map,
}

#[pymethods]
impl Map {
    #[getter]
    fn width(&self) -> usize {
        self.inner.drawing.width
    }
    #[getter]
    fn height(&self) -> usize {
        self.inner.drawing.height
    }
    #[getter]
    fn px_m(&self) -> f64 {
        self.inner.px_m
    }
    #[getter]
    fn mm_per_px(&self) -> Option<f64> {
        self.inner.mm_per_px
    }
    fn paths<'py>(&self, py: Python<'py>) -> (Bound<'py, PyBytes>, Bound<'py, PyBytes>) {
        pack(py, &self.inner.drawing.paths)
    }
    fn lines<'py>(&self, py: Python<'py>) -> (Bound<'py, PyBytes>, Bound<'py, PyBytes>) {
        pack(py, &self.inner.drawing.lines)
    }
    #[pyo3(signature = (mm_per_px=None))]
    fn svg(&self, mm_per_px: Option<f64>) -> String {
        self.inner.drawing.to_svg(mm_per_px.map(|k| k as f32))
    }
    fn png<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_png().map_err(py_err)?))
    }
    fn pen_up_travel(&self) -> f64 {
        burin_map_core::pen_up_travel(&self.inner.drawing.paths) as f64
    }
}

fn logger(log: Option<Py<PyAny>>) -> impl Fn(&burin_map::Progress) + Sync {
    move |p: &burin_map::Progress| {
        if let Some(f) = &log {
            Python::attach(|py| {
                let _ = f.call1(py, (p.to_string(),));
            });
        }
    }
}

/// (name, default, help, group) of every style parameter.
#[pyfunction]
fn style_params() -> Vec<(&'static str, f64, &'static str, &'static str)> {
    let d = Style::default();
    Style::PARAMS
        .iter()
        .map(|p| {
            (
                p.name,
                d.get(p.name).unwrap() as f32 as f64,
                p.help,
                p.group,
            )
        })
        .collect()
}

/// Parse a JSON string into a list of batch dicts.
#[pyfunction]
fn style_from_json(_py: Python<'_>, json: &str) -> PyResult<String> {
    let batches = burin_map_core::from_json_batches(json)
        .map_err(|e| PyValueError::new_err(format!("invalid --style-json: {e}")))?;
    // Return pretty-printed for Python consumption
    Ok(serde_json::to_string_pretty(&batches).unwrap_or_default())
}

/// Run a burin-map document (JavaScript) and return `{"atlas": [...],
/// "area": {...} | null}` as JSON: the batches and water passes the engine
/// draws, in order. This is what the CLI's `--script` and the web page run;
/// the interpreter is a front end and the atlas is the document.
#[pyfunction]
fn evaluate(script: &str) -> PyResult<String> {
    let document = burin_map_script::evaluate_document(script)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let mut out = serde_json::Map::new();
    out.insert("atlas".into(), document.atlas);
    out.insert("area".into(), document.area.unwrap_or(serde_json::Value::Null));
    Ok(serde_json::Value::Object(out).to_string())
}

/// (id, blurb) for each style group.
#[pyfunction]
fn style_groups() -> Vec<(&'static str, &'static str)> {
    burin_map_core::GROUPS.to_vec()
}

#[pyfunction]
fn locate(py: Python<'_>, query: &str) -> PyResult<(f64, f64, String)> {
    py.detach(|| burin_map::locate(query)).map_err(py_err)
}

#[pyfunction]
fn bounds_around(lat: f64, lon: f64, width_km: f64, height_km: f64) -> (f64, f64, f64, f64) {
    let [w, s, e, n] = burin_map::bounds_around(lat, lon, width_km, height_km);
    (w, s, e, n)
}

#[pyfunction]
fn projected_size(bounds: (f64, f64, f64, f64)) -> (f64, f64) {
    burin_map::projected_size(&[bounds.0, bounds.1, bounds.2, bounds.3])
}

/// Draw a document, `{"atlas": [...], "area": {...}}` as JSON: what
/// `evaluate` returns and `burin_map.draw()` hands over.
#[pyfunction]
#[pyo3(signature = (document, log=None))]
fn draw_document(py: Python<'_>, document: &str, log: Option<Py<PyAny>>) -> PyResult<Map> {
    let v: serde_json::Value = serde_json::from_str(document)
        .map_err(|e| PyValueError::new_err(format!("invalid document JSON: {e}")))?;
    let doc = burin_map::Document::from_atlas(&v["atlas"], v.get("area")).map_err(py_err)?;
    let log = logger(log);
    let inner = py
        .detach(|| burin_map::draw_document(&doc, log))
        .map_err(py_err)?;
    Ok(Map { inner })
}

/// Check one entry of a document — `batch`, `water`, `shore` or `area` — as
/// a document script's own calls are checked.
#[pyfunction]
fn check(kind: &str, json: &str) -> PyResult<()> {
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| PyValueError::new_err(format!("invalid JSON: {e}")))?;
    use burin_map_core::script;
    let checked = match kind {
        "batch" => script::batch_from(&v).map(drop),
        "water" => script::water_from(&v, v.get("draw").and_then(|d| d.as_str())).map(drop),
        "shore" => script::shore_from(&v).map(drop),
        "area" => script::area_from(&v).map(drop),
        other => Err(format!("nothing to check called {other:?}")),
    };
    checked.map_err(PyValueError::new_err)
}

/// The style parameters, as a document script's `burin.parameters()` lists them.
#[pyfunction]
fn parameters() -> String {
    burin_map_core::script::parameters().to_string()
}

/// `elev`: little-endian f32 heights, row 0 = north, `width` × `height`.
#[pyfunction]
#[pyo3(signature = (elev, width, height, px_m, style=None))]
fn draw_elevation(
    py: Python<'_>,
    elev: &[u8],
    width: usize,
    height: usize,
    px_m: f64,
    style: Option<&Bound<'_, PyDict>>,
) -> PyResult<Map> {
    if elev.len() != width * height * 4 {
        return Err(PyValueError::new_err(
            "elevation buffer does not match width × height",
        ));
    }
    let st = style_from(style)?;
    let e: Vec<f32> = elev
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    let drawing = py.detach(|| {
        burin_map_core::draw_batches_auto(
            &e,
            width,
            height,
            px_m as f32,
            &[Batch::along(st.clone())],
        )
    });
    Ok(Map {
        inner: burin_map::Map {
            drawing,
            style: st,
            px_m,
            mm_per_px: None,
        },
    })
}

/// Derived rasters (tx, ty, input, space) as f32 bytes, for inspection.
#[pyfunction]
#[pyo3(signature = (elev, width, height, px_m, style=None))]
#[allow(clippy::type_complexity)]
fn fields<'py>(
    py: Python<'py>,
    elev: &[u8],
    width: usize,
    height: usize,
    px_m: f64,
    style: Option<&Bound<'_, PyDict>>,
) -> PyResult<(
    Bound<'py, PyBytes>,
    Bound<'py, PyBytes>,
    Bound<'py, PyBytes>,
    Bound<'py, PyBytes>,
)> {
    if elev.len() != width * height * 4 {
        return Err(PyValueError::new_err(
            "elevation buffer does not match width × height",
        ));
    }
    let st = style_from(style)?;
    let e: Vec<f32> = elev
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    let f = py.detach(|| burin_map_core::build_fields(&e, width, height, px_m as f32, &st));
    Ok((
        f32s(py, &f.tx),
        f32s(py, &f.ty),
        f32s(py, &f.input),
        f32s(py, &f.space),
    ))
}

/// Run the `burin-map` command line with `argv` (program name first).
#[pyfunction]
fn run_cli(py: Python<'_>, argv: Vec<String>) -> PyResult<()> {
    py.detach(|| burin_map::cli::run(argv).map_err(|e| e.to_string()))
        .map_err(PyRuntimeError::new_err)
}

#[cfg(feature = "gpu")]
#[pyfunction]
fn force_cpu() {
    burin_map::engine::force_cpu();
}

/// Where the fields are built: "GPU: Apple M4 (Metal)", "CPU (forced)", …
#[cfg(feature = "gpu")]
#[pyfunction]
fn engine() -> String {
    burin_map::engine::describe()
}

#[cfg(not(feature = "gpu"))]
#[pyfunction]
fn engine() -> String {
    "CPU (built without the gpu feature)".to_string()
}

#[pymodule]
fn _burin(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<Map>()?;
    m.add_function(wrap_pyfunction!(style_params, m)?)?;
    m.add_function(wrap_pyfunction!(style_groups, m)?)?;
    m.add_function(wrap_pyfunction!(style_from_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(locate, m)?)?;
    m.add_function(wrap_pyfunction!(bounds_around, m)?)?;
    m.add_function(wrap_pyfunction!(projected_size, m)?)?;
    m.add_function(wrap_pyfunction!(draw_document, m)?)?;
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(parameters, m)?)?;
    m.add_function(wrap_pyfunction!(draw_elevation, m)?)?;
    m.add_function(wrap_pyfunction!(fields, m)?)?;
    m.add_function(wrap_pyfunction!(engine, m)?)?;
    #[cfg(feature = "gpu")]
    m.add_function(wrap_pyfunction!(force_cpu, m)?)?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    Ok(())
}

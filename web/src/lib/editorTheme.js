/**
 * A greyscale theme for the editor, in step with the black-and-white page:
 * what is what is told by tone, weight and slant, not by colour. Names (JS
 * keys, `burin` methods) are black and bold, values mid-grey, help text light
 * and italic, punctuation lightest.
 *
 * Built without inheriting from `vs`: its token rules are more specific than
 * ours (`string.value.json`, `keyword.json`, …) and would bring colour back.
 */

export const EDITOR_THEME = 'burin-grey'

/**
 * A burin document is JavaScript, with `burin` and its methods as the
 * vocabulary that matters. Monaco's own JS tokenizer does the strings,
 * numbers and comments; this adds the names.
 */
export const DOC_LANGUAGE = 'burin-doc'

export function registerDocLanguage(monaco, types) {
  if (!monaco.languages.getLanguages().some((l) => l.id === DOC_LANGUAGE)) {
    monaco.languages.register({ id: DOC_LANGUAGE })
    monaco.languages.setMonarchTokensProvider(DOC_LANGUAGE, {
      tokenizer: {
        root: [
          [/#.*$/, 'comment'],
          [/\/\*/, 'comment', '@comment'],
          [/\/\/.*$/, 'comment'],
          [/"[^"]*"|'[^']*'|`[^`]*`/, 'string'],
          [/\bburin\b/, 'type'],
          [/\b(area|batch|fill|water|shore|draw|json|parameters)\b(?=\s*\()/, 'attribute.name'],
          [/\b(elev|tone|dist|depth|lerp|rgb|clamp|smoothstep|step|abs|floor|ceil|round|min|max|mix)\b/, 'attribute.name'],
          [/\b(const|let|var|for|of|in|if|else|return|function|=>|new|throw)\b/, 'keyword'],
          [/-?\d+(\.\d+)?/, 'number'],
          [/[()[\]{},.:;=]/, 'delimiter'],
        ],
        comment: [
          [/[^*/]+/, 'comment'],
          [/\*\//, 'comment', '@pop'],
          [/./, 'comment'],
        ],
      },
    })
  }
  // The types drive hover and completion for `burin.*`, and `checkJs` makes
  // real mistakes show; syntax is checked as the document is typed.
  const ts = monaco.typescript ?? monaco.languages.typescript
  if (ts) {
    const js = ts.javascriptDefaults
    js.setCompilerOptions({
      allowJs: true,
      checkJs: true,
      allowNonTsExtensions: true,
      noEmit: true,
      target: ts.ScriptTarget.ES2020,
      lib: ['es2020'],
    })
    js.setDiagnosticsOptions({ noSemanticValidation: false, noSyntaxValidation: false })
    if (types) js.addExtraLib(types, 'file:///burin.d.ts')
  }
}

const NAME = '000000'
const VALUE = '555555'
const TEXT = '6b6b6b'
const HELP = '9a9a9a'
const MARK = 'ababab'

export function defineEditorTheme(monaco) {
  monaco.editor.defineTheme(EDITOR_THEME, {
    base: 'vs',
    inherit: false,
    rules: [
      { token: '', foreground: '1a1a1a' },
      { token: 'type', foreground: NAME, fontStyle: 'bold' },
      { token: 'string.key', foreground: NAME, fontStyle: 'bold' },
      { token: 'attribute.name', foreground: NAME, fontStyle: 'bold' },
      { token: 'number', foreground: VALUE },
      { token: 'keyword', foreground: VALUE },
      { token: 'variable', foreground: VALUE },
      { token: 'type.identifier', foreground: VALUE },
      { token: 'string', foreground: TEXT },
      { token: 'comment', foreground: HELP, fontStyle: 'italic' },
      { token: 'delimiter', foreground: MARK },
      { token: 'operators', foreground: MARK },
      { token: 'white', foreground: MARK },
    ],
    colors: {
      'editor.background': '#ffffff',
      'editor.foreground': '#1a1a1a',
      'editorGutter.background': '#ffffff',
      'editorLineNumber.foreground': '#c8c8c8',
      'editorLineNumber.activeForeground': '#1a1a1a',
      'editor.lineHighlightBackground': '#f5f5f5',
      'editor.lineHighlightBorder': '#00000000',
      'editorCursor.foreground': '#000000',
      'editor.selectionBackground': '#d9d9d9',
      'editor.inactiveSelectionBackground': '#e8e8e8',
      'editor.selectionHighlightBackground': '#ebebeb',
      'editor.wordHighlightBackground': '#ebebeb',
      'editor.wordHighlightStrongBackground': '#e0e0e0',
      'editor.findMatchBackground': '#c4c4c4',
      'editor.findMatchHighlightBackground': '#e3e3e3',
      'editorBracketHighlight.foreground1': '#ababab',
      'editorBracketHighlight.foreground2': '#ababab',
      'editorBracketHighlight.foreground3': '#ababab',
      'editorBracketHighlight.foreground4': '#ababab',
      'editorBracketHighlight.foreground5': '#ababab',
      'editorBracketHighlight.foreground6': '#ababab',
      'editorBracketMatch.background': '#e6e6e6',
      'editorBracketMatch.border': '#9a9a9a',
      'editorIndentGuide.background1': '#eeeeee',
      'editorIndentGuide.activeBackground1': '#c8c8c8',
      'editorWhitespace.foreground': '#d6d6d6',
      'editorOverviewRuler.border': '#00000000',
      'editorWidget.background': '#ffffff',
      'editorWidget.border': '#1a1a1a',
      'editorSuggestWidget.selectedBackground': '#ebebeb',
      'focusBorder': '#1a1a1a',
      'scrollbarSlider.background': '#00000018',
      'scrollbarSlider.hoverBackground': '#00000030',
      'scrollbarSlider.activeBackground': '#00000048',
    },
  })
}

/** The page's monospace (`--font-mono` in index.css), for the editor. */
export function editorFont() {
  const css = getComputedStyle(document.documentElement).getPropertyValue('--font-mono').trim()
  return css || '"IBM Plex Mono", ui-monospace, Menlo, monospace'
}

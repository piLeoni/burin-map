import { useEffect, useState } from 'react'
import { useAtom } from 'jotai'
import { UNLOCK_TTL_MS, isUnlockValid, unlockAtom } from '../state'

const PASSWORD = 'folder'

/**
 * Lightweight gate — not real security (password ships in the bundle).
 * Unlock is remembered in localStorage via Jotai for UNLOCK_TTL_MS.
 */
export default function PasswordGate({ children }) {
  const [unlock, setUnlock] = useAtom(unlockAtom)
  const [value, setValue] = useState('')
  const [error, setError] = useState(false)

  // Drop expired unlocks so we don't keep stale storage forever.
  useEffect(() => {
    if (unlock && !isUnlockValid(unlock)) setUnlock(null)
  }, [unlock, setUnlock])

  if (isUnlockValid(unlock)) return children

  const submit = (e) => {
    e.preventDefault()
    if (value === PASSWORD) {
      setUnlock({ expiresAt: Date.now() + UNLOCK_TTL_MS })
      setError(false)
      return
    }
    setError(true)
  }

  return (
    <div className="flex h-full min-h-0 items-center justify-center bg-base-100 p-6">
      <form onSubmit={submit} className="flex w-full max-w-xs flex-col gap-3 font-mono">
        <h1 className="text-2xl font-bold tracking-tight">Burin</h1>
        <input
          type="password"
          autoFocus
          className="input input-bordered w-full"
          placeholder="password"
          value={value}
          onChange={(e) => {
            setValue(e.target.value)
            setError(false)
          }}
        />
        {error ? <p className="text-xs text-error">Wrong password.</p> : null}
        <button type="submit" className="btn border border-base-content">
          Enter
        </button>
      </form>
    </div>
  )
}

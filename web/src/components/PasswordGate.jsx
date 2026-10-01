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
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <form onSubmit={submit} box-="square" className="flex w-full max-w-[40ch] flex-col gap-[1lh]">
        <h1 className="font-bold">BurinMap</h1>
        <input
          type="password"
          autoFocus
          className="w-full"
          placeholder="Password"
          value={value}
          onChange={(e) => {
            setValue(e.target.value)
            setError(false)
          }}
        />
        {error ? <p className="font-bold">✗ wrong password</p> : null}
        <button type="submit" size-="small">
          unlock
        </button>
      </form>
    </div>
  )
}

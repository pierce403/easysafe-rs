import { useMemo, useState } from 'react'

import './App.css'
import { EasysafeApi } from './lib/api'
import {
  createDocumentCommit,
  deriveWorkspaceKeys,
  emptyManifest,
  openObject,
} from './lib/crypto'
import type {
  DocumentSnapshot,
  LoadedWorkspace,
  WorkspaceHead,
  WorkspaceManifest,
} from './lib/types'
import { isoNow } from './lib/utils'

type StatusTone = 'idle' | 'success' | 'error' | 'working'

function App() {
  const [serverUrl, setServerUrl] = useState('http://127.0.0.1:8080')
  const [workspaceId, setWorkspaceId] = useState('')
  const [readPassphrase, setReadPassphrase] = useState('')
  const [writePassphrase, setWritePassphrase] = useState('')
  const [loaded, setLoaded] = useState<LoadedWorkspace | null>(null)
  const [activeId, setActiveId] = useState<string | null>(null)
  const [editorTitle, setEditorTitle] = useState('')
  const [editorBody, setEditorBody] = useState('')
  const [status, setStatus] = useState('Open a workspace published by the node.')
  const [statusTone, setStatusTone] = useState<StatusTone>('idle')
  const [busy, setBusy] = useState(false)

  const documents = loaded?.documents ?? []
  const activeDocument = useMemo(
    () => documents.find((doc) => doc.document_id === activeId) ?? null,
    [activeId, documents],
  )

  async function handleLoadWorkspace() {
    setBusy(true)
    setStatusTone('working')
    setStatus('Loading workspace from node…')
    try {
      const api = new EasysafeApi(serverUrl.trim())
      const [health, config, headRef] = await Promise.all([
        api.health(),
        api.getConfig(workspaceId.trim()),
        api.getHead(workspaceId.trim()),
      ])
      const keys = await deriveWorkspaceKeys(
        readPassphrase,
        writePassphrase || undefined,
        config,
      )

      let currentHeadId: string | null = null
      let currentHead: WorkspaceHead | null = null
      let currentManifestId: string | null = null
      let manifest: WorkspaceManifest = emptyManifest(config.workspace_id)
      let snapshots: DocumentSnapshot[] = []

      if (headRef) {
        currentHeadId = headRef.head_id
        const headBytes = await api.getObject(headRef.head_id)
        currentHead = await openObject<WorkspaceHead>(
          'workspace-head',
          headBytes,
          config,
          keys,
        )
        currentManifestId = currentHead.manifest_object_id
        const manifestBytes = await api.getObject(currentHead.manifest_object_id)
        manifest = await openObject<WorkspaceManifest>(
          'workspace-manifest',
          manifestBytes,
          config,
          keys,
        )
        snapshots = await Promise.all(
          manifest.documents.map(async (doc) => {
            const revisionBytes = await api.getObject(doc.latest_revision_id)
            return openObject<DocumentSnapshot>(
              'document-snapshot',
              revisionBytes,
              config,
              keys,
            )
          }),
        )
      }

      setLoaded({
        config,
        currentHeadId,
        currentHead,
        currentManifestId,
        manifest,
        documents: snapshots.sort((left, right) =>
          left.title.localeCompare(right.title),
        ),
        health,
        keys,
      })

      if (snapshots[0]) {
        selectDocument(snapshots[0])
      } else {
        setActiveId(null)
        setEditorTitle('')
        setEditorBody('')
      }

      setStatusTone('success')
      setStatus(
        headRef
          ? `Loaded ${snapshots.length} document(s) from workspace ${config.workspace_id}.`
          : `Workspace ${config.workspace_id} is empty. Start with a new document.`,
      )
    } catch (error) {
      setStatusTone('error')
      setStatus(error instanceof Error ? error.message : 'Failed to load workspace')
    } finally {
      setBusy(false)
    }
  }

  function selectDocument(document: DocumentSnapshot) {
    setActiveId(document.document_id)
    setEditorTitle(document.title)
    setEditorBody(document.body)
  }

  function startNewDocument() {
    setActiveId(null)
    setEditorTitle('')
    setEditorBody('')
    setStatusTone('idle')
    setStatus('Ready for a new encrypted document.')
  }

  async function handleSave() {
    if (!loaded) {
      setStatusTone('error')
      setStatus('Load a workspace first.')
      return
    }
    if (!editorTitle.trim()) {
      setStatusTone('error')
      setStatus('A document title is required.')
      return
    }

    setBusy(true)
    setStatusTone('working')
    setStatus('Encrypting and publishing revision…')
    try {
      const api = new EasysafeApi(loaded.config.server_url)
      const commit = await createDocumentCommit(
        loaded.config,
        loaded.keys,
        loaded.manifest,
        loaded.currentManifestId,
        loaded.currentHeadId,
        loaded.currentHead,
        activeId,
        editorTitle.trim(),
        editorBody,
      )

      await api.putConfig(loaded.config)
      await api.putObject(commit.revisionId, commit.revisionBytes)
      await api.putObject(commit.manifestId, commit.manifestBytes)
      await api.putObject(commit.headId, commit.headBytes)
      await api.putHead({
        workspace_id: loaded.config.workspace_id,
        head_id: commit.headId,
        updated_at: isoNow(),
      })

      const nextDocuments = loaded.documents
        .filter((doc) => doc.document_id !== commit.snapshot.document_id)
        .concat([commit.snapshot])
        .sort((left, right) => left.title.localeCompare(right.title))

      setLoaded({
        ...loaded,
        currentHeadId: commit.headId,
        currentHead: commit.nextHead,
        currentManifestId: commit.manifestId,
        manifest: commit.nextManifest,
        documents: nextDocuments,
      })
      setActiveId(commit.snapshot.document_id)
      setStatusTone('success')
      setStatus(
        `Saved "${commit.snapshot.title}" as revision ${commit.revisionId.slice(0, 12)}…`,
      )
    } catch (error) {
      setStatusTone('error')
      setStatus(error instanceof Error ? error.message : 'Failed to save document')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="app-shell">
      <aside className="control-rail">
        <div className="brand-panel">
          <p className="eyebrow">EasySafe</p>
          <h1>Encrypted documents, plain sync node.</h1>
          <p className="lede">
            This browser client opens an existing EasySafe workspace from the node,
            decrypts it locally, and publishes new encrypted revisions back to the
            same API.
          </p>
        </div>

        <section className="panel">
          <h2>Open Workspace</h2>
          <label>
            <span>Server URL</span>
            <input
              value={serverUrl}
              onChange={(event) => setServerUrl(event.target.value)}
              placeholder="http://127.0.0.1:8080"
            />
          </label>
          <label>
            <span>Workspace ID</span>
            <input
              value={workspaceId}
              onChange={(event) => setWorkspaceId(event.target.value)}
              placeholder="paste the published workspace id"
            />
          </label>
          <label>
            <span>Read Passphrase</span>
            <input
              type="password"
              value={readPassphrase}
              onChange={(event) => setReadPassphrase(event.target.value)}
              placeholder="required"
            />
          </label>
          <label>
            <span>Write Passphrase</span>
            <input
              type="password"
              value={writePassphrase}
              onChange={(event) => setWritePassphrase(event.target.value)}
              placeholder="optional if write == read"
            />
          </label>
          <button className="primary" onClick={handleLoadWorkspace} disabled={busy}>
            {busy ? 'Working…' : 'Open Workspace'}
          </button>
        </section>

        <section className="panel status-panel">
          <h2>Status</h2>
          <p className={`status-badge status-${statusTone}`}>{status}</p>
          <dl>
            <div>
              <dt>Node</dt>
              <dd>{loaded?.health.status ?? 'not checked'}</dd>
            </div>
            <div>
              <dt>Current head</dt>
              <dd>{loaded?.currentHeadId?.slice(0, 14) ?? 'none'}</dd>
            </div>
            <div>
              <dt>Documents</dt>
              <dd>{documents.length}</dd>
            </div>
            <div>
              <dt>Write access</dt>
              <dd>{loaded?.keys.writeSecretKey ? 'available' : 'read-only'}</dd>
            </div>
          </dl>
        </section>
      </aside>

      <main className="workspace-pane">
        <section className="toolbar">
          <div>
            <p className="eyebrow">Workspace</p>
            <h2>{loaded?.config.workspace_id ?? 'No workspace loaded'}</h2>
          </div>
          <div className="toolbar-actions">
            <button onClick={startNewDocument} disabled={!loaded || busy}>
              New Document
            </button>
            <button
              className="primary"
              onClick={handleSave}
              disabled={!loaded || busy || !loaded.keys.writeSecretKey}
            >
              Save Revision
            </button>
          </div>
        </section>

        <section className="editor-grid">
          <div className="document-list">
            <div className="list-header">
              <p className="eyebrow">Documents</p>
              <span>{documents.length}</span>
            </div>
            {documents.length === 0 ? (
              <p className="empty-state">
                This workspace does not have any published documents yet.
              </p>
            ) : (
              documents.map((document) => (
                <button
                  key={document.document_id}
                  className={
                    document.document_id === activeId
                      ? 'document-card active'
                      : 'document-card'
                  }
                  onClick={() => selectDocument(document)}
                >
                  <strong>{document.title}</strong>
                  <span>{new Date(document.created_at).toLocaleString()}</span>
                  <code>{document.document_id.slice(0, 12)}…</code>
                </button>
              ))
            )}
          </div>

          <div className="editor-panel">
            <div className="editor-meta">
              <p className="eyebrow">
                {activeDocument ? 'Editing Published Document' : 'Draft'}
              </p>
              <span>
                {activeDocument
                  ? `Revision from ${new Date(activeDocument.created_at).toLocaleString()}`
                  : 'Create a new encrypted document'}
              </span>
            </div>
            <input
              className="title-input"
              value={editorTitle}
              onChange={(event) => setEditorTitle(event.target.value)}
              placeholder="Document title"
            />
            <textarea
              className="body-input"
              value={editorBody}
              onChange={(event) => setEditorBody(event.target.value)}
              placeholder="Write here. Save publishes a new encrypted revision through the node."
            />
          </div>
        </section>
      </main>
    </div>
  )
}

export default App

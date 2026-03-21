import type { HeadRef, HealthResponse, WorkspaceConfig } from './types'

export class EasysafeApi {
  private readonly baseUrl: string

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl
  }

  async health(): Promise<HealthResponse> {
    return this.getJson<HealthResponse>('/api/health')
  }

  async getConfig(workspaceId: string): Promise<WorkspaceConfig> {
    return this.getJson<WorkspaceConfig>(`/api/workspaces/${workspaceId}/config`)
  }

  async putConfig(config: WorkspaceConfig): Promise<void> {
    await this.request(`/api/workspaces/${config.workspace_id}/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    })
  }

  async getHead(workspaceId: string): Promise<HeadRef | null> {
    const response = await fetch(
      `${this.baseUrl}/api/workspaces/${workspaceId}/head`,
    )
    if (response.status === 404) {
      return null
    }
    if (!response.ok) {
      throw new Error(await response.text())
    }
    return (await response.json()) as HeadRef
  }

  async putHead(head: HeadRef): Promise<void> {
    await this.request(`/api/workspaces/${head.workspace_id}/head`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(head),
    })
  }

  async getObject(objectId: string): Promise<Uint8Array> {
    const response = await fetch(`${this.baseUrl}/api/objects/${objectId}`)
    if (!response.ok) {
      throw new Error(await response.text())
    }
    return new Uint8Array(await response.arrayBuffer())
  }

  async putObject(objectId: string, bytes: Uint8Array): Promise<void> {
    const normalized = new Uint8Array(bytes.length)
    normalized.set(bytes)
    await this.request(`/api/objects/${objectId}`, {
      method: 'PUT',
      body: new Blob([normalized.buffer], { type: 'application/json' }),
    })
  }

  private async getJson<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`)
    if (!response.ok) {
      throw new Error(await response.text())
    }
    return (await response.json()) as T
  }

  private async request(path: string, init: RequestInit): Promise<void> {
    const response = await fetch(`${this.baseUrl}${path}`, init)
    if (!response.ok) {
      throw new Error(await response.text())
    }
  }
}

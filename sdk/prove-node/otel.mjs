// HSIP span processor for OpenTelemetry JS. No hard dependency on OpenTelemetry.
//
//   import { Prove } from './index.mjs'
//   import { HsipSpanProcessor } from './otel.mjs'
//   provider.addSpanProcessor(new HsipSpanProcessor(new Prove(url, key), {
//     stream: 'ai-decisions', shouldRecord: (s) => s.attributes['hsip.record'] === true }))
//
// Selected spans are fingerprinted locally; only the hash plus trace and span ids are sent.
// Recording is asynchronous and never throws into the instrumented code.

import { hashPayload } from './index.mjs'

export function spanContent(span) {
  const ctx = span.spanContext()
  const attrs = {}
  for (const [k, v] of Object.entries(span.attributes || {})) attrs[k] = String(v)
  return {
    name: span.name,
    trace_id: ctx.traceId,
    span_id: ctx.spanId,
    start: (span.startTime || []).join('.'),
    end: (span.endTime || []).join('.'),
    status: String(span.status?.code ?? 0),
    attributes: attrs,
  }
}

export class HsipSpanProcessor {
  constructor(prove, { stream, actor = 'otel', shouldRecord, maxQueue = 1000, onError = () => {} } = {}) {
    this.prove = prove
    this.stream = stream
    this.actor = actor
    this.should = shouldRecord || ((s) => s.attributes?.['hsip.record'] === true)
    this.maxQueue = maxQueue
    this.onError = onError
    this.pending = new Set()
  }

  onStart() {}

  onEnd(span) {
    try {
      if (!this.should(span) || this.pending.size >= this.maxQueue) return
      const c = spanContent(span)
      const a = c.attributes
      const p = this.prove
        .record(c, {
          stream: this.stream,
          actor: a['hsip.actor'] || this.actor,
          action: a['hsip.action'] || c.name,
          subject: a['hsip.subject'] || '',
          metadata: { trace_id: c.trace_id, span_id: c.span_id },
          payloadHash: hashPayload(c),
        })
        .catch((e) => this.onError(e))
        .finally(() => this.pending.delete(p))
      this.pending.add(p)
    } catch (e) {
      this.onError(e)
    }
  }

  async forceFlush() {
    await Promise.allSettled([...this.pending])
  }

  async shutdown() {
    await this.forceFlush()
  }
}

const BASE = import.meta.env.VITE_API_URL || '';

export async function request(method, path, body, apiKey) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: {
      'Authorization': `Bearer ${apiKey}`,
      'Content-Type': 'application/json',
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || res.statusText);
  return data;
}

/** Upload an image file. Returns { id, url, filename, content_type, size }. */
export async function uploadImage(file, apiKey) {
  const fd = new FormData();
  fd.append('file', file);
  const res = await fetch(`${BASE}/v1/uploads`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${apiKey}` },
    body: fd,
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || res.statusText);
  return data;
}

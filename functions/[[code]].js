export async function onRequestGet(context) {
  const { env, params } = context;
  const raw = params.code;
  const code = (Array.isArray(raw) ? raw.join('/') : String(raw || '')).replace(/[^a-zA-Z0-9]/g, '');

  if (!code) {
    return new Response('Not found', { status: 404 });
  }

  const supabaseUrl = (env.SUPABASE_URL || '').replace(/\/$/, '');
  const key = env.SUPABASE_SERVICE_KEY || '';
  if (!supabaseUrl || !key) {
    return new Response('Server misconfigured', { status: 500 });
  }

  const headers = { apikey: key, Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' };

  try {
    const res = await fetch(
      `${supabaseUrl}/rest/v1/urls?code=eq.${encodeURIComponent(code)}&select=original_url,clicks`,
      { headers }
    );

    if (!res.ok) {
      return new Response('Not found', { status: 404 });
    }

    const rows = await res.json();
    if (!rows.length) {
      return new Response('Short URL not found', { status: 404 });
    }

    const { original_url, clicks } = rows[0];

    fetch(
      `${supabaseUrl}/rest/v1/urls?code=eq.${encodeURIComponent(code)}`,
      {
        method: 'PATCH',
        headers: { ...headers, Prefer: 'return=minimal' },
        body: JSON.stringify({ clicks: clicks + 1 }),
      }
    ).catch(() => {});

    return Response.redirect(original_url, 302);
  } catch (err) {
    return new Response('Redirect failed', { status: 500 });
  }
}
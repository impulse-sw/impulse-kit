import { encode } from '@msgpack/msgpack';
import { endpoint } from './endpoint.js';

export async function postSignIn(xSign, userId, body) {
  const params = new URLSearchParams();
  params.set("user_id", userId);
  const response = await fetch(endpoint(`/sign-in?${params}`), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Sign": xSign
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
  return await response.json();
}

export async function patchChangePassword(xAccess, xClient, xRefresh, body) {
  const response = await fetch(endpoint("/change-password"), {
    method: "PATCH",
    headers: {
      "Content-Type": "application/msgpack",
      "X-Access": xAccess,
      "X-Client": xClient,
      "X-Refresh": xRefresh
    },
    body: encode(body),
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
}

export async function postLogout(xAccess, xClient, xRefresh) {
  const response = await fetch(endpoint("/logout"), {
    method: "POST",
    headers: {
      "X-Access": xAccess,
      "X-Client": xClient,
      "X-Refresh": xRefresh
    },
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
}

export async function deleteAccount(xAccess, xClient, xRefresh, body) {
  const response = await fetch(endpoint("/account"), {
    method: "DELETE",
    headers: {
      "Content-Type": "application/json",
      "X-Access": xAccess,
      "X-Client": xClient,
      "X-Refresh": xRefresh
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
}

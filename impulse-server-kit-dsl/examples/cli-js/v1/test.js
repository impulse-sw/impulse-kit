import { decode } from '@msgpack/msgpack';
import { endpoint } from './endpoint.js';

export async function getTest() {
  const response = await fetch(endpoint("/test"), {
    method: "GET",
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
}

export async function postAudio(audio) {
  const formData = new FormData();
  formData.append("audio", new Blob([audio]));
  const response = await fetch(endpoint("/audio"), {
    method: "POST",
    body: formData,
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
  return decode(new Uint8Array(await response.arrayBuffer()));
}

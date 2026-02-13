import { endpoint } from './endpoint.js';

export async function getChats(chatId) {
  const params = new URLSearchParams();
  params.set("chat_id", chatId);
  const response = await fetch(endpoint(`/chats?${params}`), {
    method: "GET",
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
  return await response.json();
}

export async function getChatById(id) {
  const response = await fetch(endpoint(`/chat/${id}`), {
    method: "GET",
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
  return await response.json();
}

export async function postChatByIdAudioRequest(id, body) {
  const formData = new FormData();
  formData.append("audio", new Blob([body]));
  const response = await fetch(endpoint(`/chat/${id}/audio-request`), {
    method: "POST",
    body: formData,
  });
  if (!response.ok) {
    const err = await response.text();
    throw new Error(err);
  }
}

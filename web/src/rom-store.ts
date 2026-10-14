/**
 * Keeps the ROM files the player loaded in IndexedDB, so the game is ready
 * on the next visit. Nothing is ever uploaded.
 */

/** A ROM file or archive as the player provided it. */
export interface StoredFile {
  readonly name: string;
  readonly data: ArrayBuffer;
}

const DATABASE_NAME = "midway-racer";
const DATABASE_VERSION = 1;
const STORE_NAME = "rom-files";

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DATABASE_NAME, DATABASE_VERSION);
    request.onupgradeneeded = () => {
      request.result.createObjectStore(STORE_NAME, { keyPath: "name" });
    };
    request.onsuccess = () => {
      resolve(request.result);
    };
    request.onerror = () => {
      reject(request.error ?? new Error("IndexedDB is unavailable"));
    };
  });
}

function finished(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => {
      resolve();
    };
    transaction.onerror = () => {
      reject(transaction.error ?? new Error("IndexedDB transaction failed"));
    };
    transaction.onabort = () => {
      reject(transaction.error ?? new Error("IndexedDB transaction aborted"));
    };
  });
}

/** Returns the stored files, oldest first. */
export async function loadFiles(): Promise<StoredFile[]> {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readonly");
    const request = transaction.objectStore(STORE_NAME).getAll();
    await finished(transaction);
    return request.result as StoredFile[];
  } finally {
    database.close();
  }
}

/** Replaces the stored files with `files`. */
export async function saveFiles(files: readonly StoredFile[]): Promise<void> {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readwrite");
    const store = transaction.objectStore(STORE_NAME);
    store.clear();
    for (const file of files) {
      store.put(file);
    }
    await finished(transaction);
  } finally {
    database.close();
  }
}

/** Forgets every stored file. */
export async function clearFiles(): Promise<void> {
  await saveFiles([]);
}

import { moduleNative } from '@arthex/sdk';
import type { AlgorithmId, PasswordKind } from './algorithms';
import { ID } from './identity';

export const native = moduleNative(ID);

export interface Digest {
  id: AlgorithmId;
  hex: string;
}

/** A digest in a list, before its value is known. */
export interface Entry {
  id: AlgorithmId;
  hex: string | null;
}

export interface FileState {
  job: number;
  name: string;
  size: number;
  done: number;
  digests: Digest[] | null;
  failed: boolean;
}

/** Every digest of a text, or its HMAC with `key` for the algorithms that have one. */
export const digestsOf = (text: string, key?: string) =>
  native.call<Digest[]>('digest', key === undefined ? { text } : { text, key });

export const hashPassword = (kind: PasswordKind, password: string) =>
  native.call<string>('passwordHash', { kind, password });

/** Rejects a hash the program does not know. */
export const checkPassword = (password: string, hash: string) =>
  native.call<boolean>('passwordVerify', { password, hash });

/** The file the program hashes, chosen in the tool or from the right-click menu. It goes on with the overlay closed. */
class FileHash {
  state = $state<FileState | null>(null);
  private listening = false;

  async load(): Promise<void> {
    if (!this.listening) {
      this.listening = true;
      native.on('file', (payload) => (this.state = payload as FileState));
    }
    this.state = await native.call<FileState | null>('fileState');
  }

  async pick(): Promise<void> {
    const picked = await native.call<FileState | null>('pickFile');
    if (picked) this.state = picked;
  }

  cancel(): void {
    this.state = null;
    void native.call('cancelFile');
  }
}

export const fileHash = new FileHash();

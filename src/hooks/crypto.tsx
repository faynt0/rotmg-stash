import { useSettings } from '@providers/SettingsProvider';
import * as crypto from 'crypto';

const ALGORITHM = 'aes-256-cbc';
const IV_LENGTH = 16; // For AES, this is always 16
const KEY_LENGTH = 32; // For aes-256-cbc, this is 32

/**
 * Derives a key of a specific length from the input secret.
 * @param secret The secret string to derive the key from.
 * @returns A Buffer containing the derived key.
 */
const deriveKey = (secret: string): Buffer => {
  return crypto.createHash('sha256').update(String(secret)).digest();
};

/**
 * Custom hook that provides encryption and decryption functionalities using AES-256-CBC algorithm
 * from Node.js's crypto module.
 *
 * @returns {Object} An object containing two functions:
 * - `encrypt`: Encrypts a given text using a secret key from settings.
 * - `decrypt`: Decrypts a given ciphertext using a secret key from settings.
 *
 * @example
 * const { encrypt, decrypt } = useCrypto();
 * const encryptedText = encrypt("Hello B)");
 * const decryptedText = decrypt(encryptedText); // decryptedText will be "Hello B)" or null if decryption fails
 */
const useCrypto = () => {
  const { settings } = useSettings();

  const getDerivedKey = (): Buffer | null => {
    if (!settings?.secret_key) {
      console.error("Secret key is not set.");
      return deriveKey("");
    }
    return deriveKey(settings.secret_key);
  };

  return {
    encrypt: (text: string): string | null => {
      const key = getDerivedKey();
      if (!key) return null;

      try {
        const iv = crypto.randomBytes(IV_LENGTH);
        const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
        let encrypted = cipher.update(text, 'utf8', 'hex');
        encrypted += cipher.final('hex');
        return iv.toString('hex') + ':' + encrypted;
      } catch (error) {
        console.error("Encryption failed:", error);
        return null;
      }
    },
    decrypt: (text: string): string | null => {
      const key = getDerivedKey();
      if (!key) return null;

      try {
        const textParts = text.split(':');
        if (textParts.length !== 2) {
          console.error("Invalid ciphertext format. Expected iv:ciphertext");
          return null;
        }
        const iv = Buffer.from(textParts.shift()!, 'hex');
        const encryptedText = textParts.join(':');
        
        if (iv.length !== IV_LENGTH) {
          console.error(`Invalid IV length. Expected ${IV_LENGTH} bytes but got ${iv.length}.`);
          return null;
        }

        const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
        let decrypted = decipher.update(encryptedText, 'hex', 'utf8');
        decrypted += decipher.final('utf8');
        console.log("Decrypted text:", decrypted);
        return decrypted;
      } catch (error) {
        console.error("Decryption failed:", error);
        return null;
      }
    }
  };
};

export default useCrypto;

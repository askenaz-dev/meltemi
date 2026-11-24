// SPDX-License-Identifier: Apache-2.0

package dev.meltemi.vault

import android.app.Activity
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Bytes, as lowercase hex, which is how they travel in both directions. */
@InvokeArg
class Bytes {
  lateinit var data: String
}

/**
 * Seals and opens bytes with an AES-256-GCM key that lives in the Android
 * keystore and never leaves it. The app keeps the sealed bytes; the keystore
 * keeps the key; neither side ever holds the other's half.
 */
@TauriPlugin
class VaultPlugin(private val activity: Activity) : Plugin(activity) {
  private val alias = "meltemi-companion-vault"
  private val ivBytes = 12
  private val tagBits = 128

  private fun key(): SecretKey {
    val store = KeyStore.getInstance("AndroidKeyStore")
    store.load(null)
    (store.getKey(alias, null) as? SecretKey)?.let { return it }
    val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
    generator.init(
      KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setKeySize(256)
        // The daemon's deny-by-default is the gate; a biometric prompt on
        // every launch would be theatre in front of it.
        .setUserAuthenticationRequired(false)
        .build()
    )
    return generator.generateKey()
  }

  @Command
  fun seal(invoke: Invoke) {
    try {
      val plain = invoke.parseArgs(Bytes::class.java).data.hexToBytes()
      val cipher = Cipher.getInstance("AES/GCM/NoPadding")
      cipher.init(Cipher.ENCRYPT_MODE, key())
      val sealed = cipher.iv + cipher.doFinal(plain)
      val out = JSObject()
      out.put("data", sealed.toHex())
      invoke.resolve(out)
    } catch (e: Exception) {
      invoke.reject("the keystore refused to seal: ${e.message}")
    }
  }

  @Command
  fun open(invoke: Invoke) {
    try {
      val sealed = invoke.parseArgs(Bytes::class.java).data.hexToBytes()
      require(sealed.size > ivBytes) { "the sealed bytes are shorter than their own nonce" }
      val iv = sealed.copyOfRange(0, ivBytes)
      val cipher = Cipher.getInstance("AES/GCM/NoPadding")
      cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(tagBits, iv))
      val plain = cipher.doFinal(sealed, ivBytes, sealed.size - ivBytes)
      val out = JSObject()
      out.put("data", plain.toHex())
      invoke.resolve(out)
    } catch (e: Exception) {
      invoke.reject("the keystore refused to open: ${e.message}")
    }
  }
}

private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

private fun String.hexToBytes(): ByteArray {
  require(length % 2 == 0) { "odd-length hex" }
  return ByteArray(length / 2) { i -> substring(2 * i, 2 * i + 2).toInt(16).toByte() }
}

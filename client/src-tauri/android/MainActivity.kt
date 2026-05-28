package com.alaydriem.bvc.client

import android.content.Context
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  // Resolved by Java_com_alaydriem_bvc_client_MainActivity_initNdkContext in
  // src-tauri/src/lib.rs. Tauri 2.11 / tao 0.35 stopped initializing the
  // global ndk-context (tauri-apps/tao#1154), so we populate it ourselves
  // with the application context before any plugin touches JNI on a worker
  // thread (cpal, tauri-plugin-keyring, webbrowser, ...).
  private external fun initNdkContext(context: Context)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    initNdkContext(this.applicationContext)
  }
}

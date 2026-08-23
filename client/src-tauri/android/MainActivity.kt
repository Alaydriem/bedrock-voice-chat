package com.alaydriem.bvc.client

import android.content.Context
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  companion object {
    private const val VOID_BACKGROUND = 0xFF1C1132.toInt()

    private const val INSET_FRACTION = 0.65f
  }

  private external fun initNdkContext(context: Context)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    window.setBackgroundDrawable(ColorDrawable(VOID_BACKGROUND))
    initNdkContext(this.applicationContext)
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.setBackgroundColor(VOID_BACKGROUND)

    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, windowInsets ->
      val types =
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      // Both types, so each edge takes whichever is deeper. Reserving the status bar alone
      // was tried: on the device this was measured against the two are identical, because
      // the OS had already sized the bar to clear a 45.7dp camera cutout, so it changed
      // nothing and gave up the cutout's protection wherever the two disagree.
      val insets = windowInsets.getInsets(types)

      // A margin, not padding. Padding lives inside the view's own bounds, so Chromium
      // still lays the page out against the full window and the reservation is invisible
      // to CSS -- `setPadding` here reserved 160px that the page never saw, and a 48dp
      // header's controls landed on the clock. A margin makes the view itself smaller,
      // and the viewport is the view.
      val left = (insets.left * INSET_FRACTION).toInt()
      val top = (insets.top * INSET_FRACTION).toInt()
      val right = (insets.right * INSET_FRACTION).toInt()
      val bottom = (insets.bottom * INSET_FRACTION).toInt()

      val params = view.layoutParams as? ViewGroup.MarginLayoutParams
      if (
        params != null &&
        (
          params.topMargin != top ||
          params.bottomMargin != bottom ||
          params.leftMargin != left ||
          params.rightMargin != right
        )
      ) {
        params.setMargins(left, top, right, bottom)
        view.layoutParams = params
      }

      val density = view.resources.displayMetrics.density

      // Installing a listener replaces the view's own onApplyWindowInsets, so the result
      // still has to be handed to it or Chromium is cut out of inset handling entirely,
      // `ime()` included.
      val withoutBars = WindowInsetsCompat.Builder(windowInsets)
        .setInsets(types, Insets.NONE)
        .build()

      ViewCompat.onApplyWindowInsets(view, withoutBars)
    }
  }
}

package com.rotmg_stash.app

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {

    companion object {
        private const val STORAGE_PERMISSION_CODE = 101
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        checkAndRequestPermissions()
    }

    private fun checkAndRequestPermissions() {
        val permissionsToRequest = mutableListOf<String>()

        // For Android 10 (API 29) and above, WRITE_EXTERNAL_STORAGE for app's own files
        // in external storage is not strictly needed if using scoped storage properly.
        // However, for broader compatibility or if accessing shared storage, it might be.
        // READ_EXTERNAL_STORAGE is generally needed to read any external files.
        // For internal storage (/data/data/your.package/files), no permissions are needed.

        if (ContextCompat.checkSelfPermission(this, Manifest.permission.READ_EXTERNAL_STORAGE)
            != PackageManager.PERMISSION_GRANTED
        ) {
            permissionsToRequest.add(Manifest.permission.READ_EXTERNAL_STORAGE)
        }

        // WRITE_EXTERNAL_STORAGE is effectively granted if you target API 29+ and only write to app-specific directories.
        // For older versions or writing to shared storage (pre-API 29), it's needed.
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) { // Or if you need to write to shared storage
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.WRITE_EXTERNAL_STORAGE)
                != PackageManager.PERMISSION_GRANTED
            ) {
                permissionsToRequest.add(Manifest.permission.WRITE_EXTERNAL_STORAGE)
            }
        }


        if (permissionsToRequest.isNotEmpty()) {
            ActivityCompat.requestPermissions(
                this,
                permissionsToRequest.toTypedArray(),
                STORAGE_PERMISSION_CODE
            )
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == STORAGE_PERMISSION_CODE) {
            if (grantResults.isNotEmpty()) {
                permissions.forEachIndexed { index, permission ->
                    if (grantResults[index] == PackageManager.PERMISSION_GRANTED) {
                        // Log or handle granted permission
                        println("Permission granted: $permission")
                    } else {
                        // Log or handle denied permission
                        // You might want to explain to the user why the permission is needed
                        println("Permission denied: $permission")
                    }
                }
            }
        }
    }
}
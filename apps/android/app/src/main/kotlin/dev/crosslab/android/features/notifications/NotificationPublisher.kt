package dev.crosslab.android.features.notifications

/** Narrow OS listener boundary. The session agent still owns all authorization. */
interface NotificationPublisher {
    fun canCapture(): Boolean

    fun contentAllowed(): Boolean

    fun posted(
        key: String,
        appLabel: String,
        title: String?,
        preview: String?,
        protected: Boolean,
    ): Boolean

    fun removed(key: String): Boolean

    fun disableNotifications()
}

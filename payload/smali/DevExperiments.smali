.class public final Ldanger/DevExperiments;
.super Ljava/lang/Object;
.source "DangerPatcher"

.field public static a:Landroid/content/Context;

.method public static init(Landroid/content/Context;)V
    .locals 1
    invoke-virtual {p0}, Landroid/content/Context;->getApplicationContext()Landroid/content/Context;
    move-result-object v0
    sput-object v0, Ldanger/DevExperiments;->a:Landroid/content/Context;
    return-void
.end method

.method public static getOverride(Ljava/lang/String;)Ljava/lang/String;
    .locals 6
    sget-object v0, Ldanger/DevExperiments;->a:Landroid/content/Context;
    if-nez v0, :ready
    const/4 v0, 0x0
    return-object v0
    :ready
    const-string v1, "danger_dev_experiments"
    const/4 v2, 0x0
    invoke-virtual {v0, v1, v2}, Landroid/content/Context;->getSharedPreferences(Ljava/lang/String;I)Landroid/content/SharedPreferences;
    move-result-object v0
    const-string v1, "__server__"
    invoke-interface {v0, p0, v1}, Landroid/content/SharedPreferences;->getString(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v2
    move-object v3, p0
    invoke-virtual {v1, v2}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v4
    if-eqz v4, :return_override
    const-string v4, "_NotReady"
    invoke-virtual {p0, v4}, Ljava/lang/String;->endsWith(Ljava/lang/String;)Z
    move-result v5
    if-eqz v5, :global_force
    invoke-virtual {p0}, Ljava/lang/String;->length()I
    move-result v5
    add-int/lit8 v5, v5, -0x9
    const/4 v4, 0x0
    invoke-virtual {p0, v4, v5}, Ljava/lang/String;->substring(II)Ljava/lang/String;
    move-result-object v3
    invoke-interface {v0, v3, v1}, Landroid/content/SharedPreferences;->getString(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v2
    invoke-virtual {v1, v2}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v4
    if-eqz v4, :return_override
    :global_force
    const-string v4, "danger_force_all"
    const-string v5, "__server__"
    invoke-interface {v0, v4, v5}, Landroid/content/SharedPreferences;->getString(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v2
    invoke-virtual {v1, v2}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v4
    if-nez v4, :none
    :return_override
    return-object v2
    :none
    const/4 v0, 0x0
    return-object v0
.end method

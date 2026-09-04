.class public final Ldanger/OpenDiscordPresence;
.super Ljava/lang/Object;
.source "Ympatcher"

.implements Lkotlin/jvm/functions/Function0;

.field public static final a:Ldanger/OpenDiscordPresence;

.method static constructor <clinit>()V
    .locals 1
    new-instance v0, Ldanger/OpenDiscordPresence;
    invoke-direct {v0}, Ldanger/OpenDiscordPresence;-><init>()V
    sput-object v0, Ldanger/OpenDiscordPresence;->a:Ldanger/OpenDiscordPresence;
    return-void
.end method

.method public constructor <init>()V
    .locals 0
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    return-void
.end method

.method public invoke()Ljava/lang/Object;
    .locals 3
    new-instance v0, Landroid/content/Intent;
    sget-object v1, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const-class v2, Ldev/pyanexy/ympresence/DiscordPresenceActivity;
    invoke-direct {v0, v1, v2}, Landroid/content/Intent;-><init>(Landroid/content/Context;Ljava/lang/Class;)V
    const/high16 v2, 0x10000000
    invoke-virtual {v0, v2}, Landroid/content/Intent;->addFlags(I)Landroid/content/Intent;
    invoke-virtual {v1, v0}, Landroid/content/Context;->startActivity(Landroid/content/Intent;)V
    sget-object v0, Lkotlin/Unit;->a:Lkotlin/Unit;
    return-object v0
.end method

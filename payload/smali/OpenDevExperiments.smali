.class public final Ldanger/OpenDevExperiments;
.super Ljava/lang/Object;
.source "DangerPatcher"
.implements Lkotlin/jvm/functions/Function0;

.field public static final a:Ldanger/OpenDevExperiments;

.method static constructor <clinit>()V
    .locals 1
    new-instance v0, Ldanger/OpenDevExperiments;
    invoke-direct {v0}, Ldanger/OpenDevExperiments;-><init>()V
    sput-object v0, Ldanger/OpenDevExperiments;->a:Ldanger/OpenDevExperiments;
    return-void
.end method

.method private constructor <init>()V
    .locals 0
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    return-void
.end method

.method public invoke()Ljava/lang/Object;
    .locals 4
    sget-object v0, Ldanger/DevExperiments;->a:Landroid/content/Context;
    if-eqz v0, :done
    new-instance v1, Landroid/content/Intent;
    const-class v2, Ldanger/DevExperimentsActivity;
    invoke-direct {v1, v0, v2}, Landroid/content/Intent;-><init>(Landroid/content/Context;Ljava/lang/Class;)V
    const/high16 v3, 0x10000000
    invoke-virtual {v1, v3}, Landroid/content/Intent;->addFlags(I)Landroid/content/Intent;
    invoke-virtual {v0, v1}, Landroid/content/Context;->startActivity(Landroid/content/Intent;)V
    :done
    sget-object v0, Lkotlin/Unit;->a:Lkotlin/Unit;
    return-object v0
.end method

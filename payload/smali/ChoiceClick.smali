.class public final Ldanger/ChoiceClick;
.super Ljava/lang/Object;
.source "DangerPatcher"
.implements Landroid/view/View$OnClickListener;

.field private final a:Ldanger/DevExperimentsActivity;
.field private final b:Landroid/app/Dialog;
.field private final c:Ljava/lang/String;
.field private final d:Ljava/lang/String;

.method public constructor <init>(Ldanger/DevExperimentsActivity;Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;)V
    .locals 0
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    iput-object p1, p0, Ldanger/ChoiceClick;->a:Ldanger/DevExperimentsActivity;
    iput-object p2, p0, Ldanger/ChoiceClick;->b:Landroid/app/Dialog;
    iput-object p3, p0, Ldanger/ChoiceClick;->c:Ljava/lang/String;
    iput-object p4, p0, Ldanger/ChoiceClick;->d:Ljava/lang/String;
    return-void
.end method

.method public onClick(Landroid/view/View;)V
    .locals 4
    iget-object v0, p0, Ldanger/ChoiceClick;->a:Ldanger/DevExperimentsActivity;
    iget-object v1, p0, Ldanger/ChoiceClick;->c:Ljava/lang/String;
    iget-object v2, p0, Ldanger/ChoiceClick;->d:Ljava/lang/String;
    iget-object v3, p0, Ldanger/ChoiceClick;->b:Landroid/app/Dialog;
    invoke-virtual {v0, v1, v2, v3}, Ldanger/DevExperimentsActivity;->applyChoice(Ljava/lang/String;Ljava/lang/String;Landroid/app/Dialog;)V
    return-void
.end method

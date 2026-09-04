.class public final Ldanger/AboutActivity;
.super Landroid/app/Activity;
.source "Ympatcher"

.implements Landroid/view/View$OnClickListener;

.method public constructor <init>()V
    .locals 0
    invoke-direct {p0}, Landroid/app/Activity;-><init>()V
    return-void
.end method

.method private r(Ljava/lang/String;Ljava/lang/String;)I
    .locals 2
    invoke-virtual {p0}, Landroid/content/Context;->getResources()Landroid/content/res/Resources;
    move-result-object v0
    invoke-virtual {p0}, Landroid/content/Context;->getPackageName()Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, p1, p2, v1}, Landroid/content/res/Resources;->getIdentifier(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I
    move-result v0
    return v0
.end method

.method private v(Ljava/lang/String;)Landroid/view/View;
    .locals 2
    const-string v0, "id"
    invoke-direct {p0, p1, v0}, Ldanger/AboutActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v1
    invoke-virtual {p0, v1}, Landroid/app/Activity;->findViewById(I)Landroid/view/View;
    move-result-object v0
    return-object v0
.end method

.method protected onCreate(Landroid/os/Bundle;)V
    .locals 2
    invoke-super {p0, p1}, Landroid/app/Activity;->onCreate(Landroid/os/Bundle;)V
    const/4 v0, 0x1
    invoke-virtual {p0, v0}, Landroid/app/Activity;->requestWindowFeature(I)Z
    const-string v0, "danger_about_sheet"
    const-string v1, "layout"
    invoke-direct {p0, v0, v1}, Ldanger/AboutActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v0
    invoke-virtual {p0, v0}, Landroid/app/Activity;->setContentView(I)V

    const-string v0, "danger_about_scrim"
    invoke-direct {p0, v0}, Ldanger/AboutActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    invoke-virtual {v0, p0}, Landroid/view/View;->setOnClickListener(Landroid/view/View$OnClickListener;)V

    const-string v0, "danger_about_close"
    invoke-direct {p0, v0}, Ldanger/AboutActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    invoke-virtual {v0, p0}, Landroid/view/View;->setOnClickListener(Landroid/view/View$OnClickListener;)V

    const-string v0, "danger_about_github"
    invoke-direct {p0, v0}, Ldanger/AboutActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    invoke-virtual {v0, p0}, Landroid/view/View;->setOnClickListener(Landroid/view/View$OnClickListener;)V
    return-void
.end method

.method public onClick(Landroid/view/View;)V
    .locals 4
    invoke-virtual {p1}, Landroid/view/View;->getId()I
    move-result v0
    const-string v1, "danger_about_github"
    invoke-direct {p0, v1}, Ldanger/AboutActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v1
    invoke-virtual {v1}, Landroid/view/View;->getId()I
    move-result v1
    if-ne v0, v1, :close

    new-instance v0, Landroid/content/Intent;
    const-string v1, "android.intent.action.VIEW"
    const-string v2, "https://github.com/pyanexya/ympatcher"
    invoke-static {v2}, Landroid/net/Uri;->parse(Ljava/lang/String;)Landroid/net/Uri;
    move-result-object v2
    invoke-direct {v0, v1, v2}, Landroid/content/Intent;-><init>(Ljava/lang/String;Landroid/net/Uri;)V
    const/high16 v3, 0x10000000
    invoke-virtual {v0, v3}, Landroid/content/Intent;->addFlags(I)Landroid/content/Intent;
    invoke-virtual {p0, v0}, Landroid/content/Context;->startActivity(Landroid/content/Intent;)V
    return-void

    :close
    invoke-virtual {p0}, Landroid/app/Activity;->finish()V
    return-void
.end method

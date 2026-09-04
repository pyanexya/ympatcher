.class public final Ldanger/DevExperimentsActivity;
.super Landroid/app/Activity;
.source "DangerPatcher"

.implements Landroid/text/TextWatcher;
.implements Landroid/widget/AdapterView$OnItemClickListener;
.implements Landroid/view/View$OnTouchListener;
.implements Landroid/view/View$OnClickListener;

.field private a:Landroid/widget/ArrayAdapter;
.field private b:Landroid/content/SharedPreferences;
.field private c:Landroid/view/View;
.field private d:F

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
    invoke-direct {p0, p1, v0}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v1
    invoke-virtual {p0, v1}, Landroid/app/Activity;->findViewById(I)Landroid/view/View;
    move-result-object v0
    return-object v0
.end method

.method private s(Ljava/lang/String;)Ljava/lang/String;
    .locals 2
    const-string v0, "string"
    invoke-direct {p0, p1, v0}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v0
    invoke-virtual {p0, v0}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v1
    return-object v1
.end method

.method protected onCreate(Landroid/os/Bundle;)V
    .locals 10
    invoke-super {p0, p1}, Landroid/app/Activity;->onCreate(Landroid/os/Bundle;)V
    invoke-static {p0}, Ldanger/DevExperiments;->init(Landroid/content/Context;)V

    const/4 v0, 0x1
    invoke-virtual {p0, v0}, Landroid/app/Activity;->requestWindowFeature(I)Z

    const-string v0, "danger_dev_sheet"
    const-string v1, "layout"
    invoke-direct {p0, v0, v1}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v0
    invoke-virtual {p0, v0}, Landroid/app/Activity;->setContentView(I)V

    const-string v0, "danger_dev_experiments"
    const/4 v1, 0x0
    invoke-virtual {p0, v0, v1}, Landroid/content/Context;->getSharedPreferences(Ljava/lang/String;I)Landroid/content/SharedPreferences;
    move-result-object v0
    iput-object v0, p0, Ldanger/DevExperimentsActivity;->b:Landroid/content/SharedPreferences;

    const-string v0, "danger_sheet"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    iput-object v0, p0, Ldanger/DevExperimentsActivity;->c:Landroid/view/View;

    const-string v0, "danger_drag_area"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    invoke-virtual {v0, p0}, Landroid/view/View;->setOnTouchListener(Landroid/view/View$OnTouchListener;)V

    const-string v0, "danger_search"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    check-cast v0, Landroid/widget/EditText;
    invoke-virtual {v0, p0}, Landroid/widget/TextView;->addTextChangedListener(Landroid/text/TextWatcher;)V

    const-string v0, "danger_force_all"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    invoke-virtual {v0, p0}, Landroid/view/View;->setOnClickListener(Landroid/view/View$OnClickListener;)V

    const-string v0, "danger_reset"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    invoke-virtual {v0, p0}, Landroid/view/View;->setOnClickListener(Landroid/view/View$OnClickListener;)V

    invoke-virtual {p0}, Landroid/content/Context;->getResources()Landroid/content/res/Resources;
    move-result-object v0
    const-string v1, "danger_experiment_names"
    const-string v2, "array"
    invoke-direct {p0, v1, v2}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v1
    invoke-virtual {v0, v1}, Landroid/content/res/Resources;->getStringArray(I)[Ljava/lang/String;
    move-result-object v6

    new-instance v7, Landroid/widget/ArrayAdapter;
    const-string v0, "danger_experiment_row"
    const-string v1, "layout"
    invoke-direct {p0, v0, v1}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v8
    const-string v0, "danger_experiment_name"
    const-string v1, "id"
    invoke-direct {p0, v0, v1}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v9
    invoke-direct {v7, p0, v8, v9, v6}, Landroid/widget/ArrayAdapter;-><init>(Landroid/content/Context;II[Ljava/lang/Object;)V
    iput-object v7, p0, Ldanger/DevExperimentsActivity;->a:Landroid/widget/ArrayAdapter;

    const-string v0, "danger_experiment_list"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    check-cast v0, Landroid/widget/ListView;
    invoke-virtual {v0, v7}, Landroid/widget/AdapterView;->setAdapter(Landroid/widget/Adapter;)V
    invoke-virtual {v0, p0}, Landroid/widget/AdapterView;->setOnItemClickListener(Landroid/widget/AdapterView$OnItemClickListener;)V

    const-string v0, "danger_experiment_count"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v0
    check-cast v0, Landroid/widget/TextView;
    new-instance v1, Ljava/lang/StringBuilder;
    invoke-direct {v1}, Ljava/lang/StringBuilder;-><init>()V
    array-length v2, v6
    invoke-virtual {v1, v2}, Ljava/lang/StringBuilder;->append(I)Ljava/lang/StringBuilder;
    const-string v2, "danger_experiment_count_suffix"
    invoke-direct {p0, v2}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v2
    invoke-virtual {v1, v2}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    invoke-virtual {v1}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Landroid/widget/TextView;->setText(Ljava/lang/CharSequence;)V

    invoke-direct {p0}, Ldanger/DevExperimentsActivity;->updateForce()V
    return-void
.end method

.method private updateForce()V
    .locals 5
    iget-object v0, p0, Ldanger/DevExperimentsActivity;->b:Landroid/content/SharedPreferences;
    const-string v1, "danger_force_all"
    const-string v2, "__server__"
    invoke-interface {v0, v1, v2}, Landroid/content/SharedPreferences;->getString(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v0
    const-string v1, "danger_force_all"
    invoke-direct {p0, v1}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v1
    check-cast v1, Landroid/widget/TextView;
    const-string v2, "__server__"
    invoke-virtual {v2, v0}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v3
    if-eqz v3, :danger_force_value
    const-string v2, "danger_force_all_server"
    invoke-direct {p0, v2}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v0
    goto :danger_force_set
    :danger_force_value
    new-instance v2, Ljava/lang/StringBuilder;
    invoke-direct {v2}, Ljava/lang/StringBuilder;-><init>()V
    const-string v3, "danger_force_all_value"
    invoke-direct {p0, v3}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v3
    invoke-virtual {v2, v3}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    invoke-virtual {v0}, Ljava/lang/String;->toUpperCase()Ljava/lang/String;
    move-result-object v0
    invoke-virtual {v2, v0}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    invoke-virtual {v2}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v0
    :danger_force_set
    invoke-virtual {v1, v0}, Landroid/widget/TextView;->setText(Ljava/lang/CharSequence;)V
    return-void
.end method

.method public onClick(Landroid/view/View;)V
    .locals 2
    invoke-virtual {p1}, Landroid/view/View;->getId()I
    move-result v0
    const-string v1, "danger_reset"
    invoke-direct {p0, v1}, Ldanger/DevExperimentsActivity;->v(Ljava/lang/String;)Landroid/view/View;
    move-result-object v1
    invoke-virtual {v1}, Landroid/view/View;->getId()I
    move-result v1
    if-ne v0, v1, :danger_force_click
    invoke-direct {p0}, Ldanger/DevExperimentsActivity;->resetDefaults()V
    return-void
    :danger_force_click
    const-string v0, "danger_force_all"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->showChoice(Ljava/lang/String;)V
    return-void
.end method

.method public onItemClick(Landroid/widget/AdapterView;Landroid/view/View;IJ)V
    .locals 1
    iget-object v0, p0, Ldanger/DevExperimentsActivity;->a:Landroid/widget/ArrayAdapter;
    invoke-virtual {v0, p3}, Landroid/widget/ArrayAdapter;->getItem(I)Ljava/lang/Object;
    move-result-object v0
    check-cast v0, Ljava/lang/String;
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->showChoice(Ljava/lang/String;)V
    return-void
.end method

.method private bind(Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V
    .locals 3
    const-string v0, "id"
    invoke-direct {p0, p2, v0}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v0
    invoke-virtual {p1, v0}, Landroid/app/Dialog;->findViewById(I)Landroid/view/View;
    move-result-object v0
    iget-object v1, p0, Ldanger/DevExperimentsActivity;->b:Landroid/content/SharedPreferences;
    const-string v2, "__server__"
    invoke-interface {v1, p4, v2}, Landroid/content/SharedPreferences;->getString(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v1
    invoke-virtual {p3, v1}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v1
    invoke-virtual {v0, v1}, Landroid/view/View;->setSelected(Z)V
    new-instance v1, Ldanger/ChoiceClick;
    invoke-direct {v1, p0, p1, p4, p3}, Ldanger/ChoiceClick;-><init>(Ldanger/DevExperimentsActivity;Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;)V
    invoke-virtual {v0, v1}, Landroid/view/View;->setOnClickListener(Landroid/view/View$OnClickListener;)V
    return-void
.end method

.method private showChoice(Ljava/lang/String;)V
    .locals 5
    new-instance v0, Landroid/app/Dialog;
    invoke-direct {v0, p0}, Landroid/app/Dialog;-><init>(Landroid/content/Context;)V
    const/4 v1, 0x1
    invoke-virtual {v0, v1}, Landroid/app/Dialog;->requestWindowFeature(I)Z
    const-string v1, "danger_choice_dialog"
    const-string v2, "layout"
    invoke-direct {p0, v1, v2}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v1
    invoke-virtual {v0, v1}, Landroid/app/Dialog;->setContentView(I)V

    const-string v1, "danger_choice_title"
    const-string v2, "id"
    invoke-direct {p0, v1, v2}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v1
    invoke-virtual {v0, v1}, Landroid/app/Dialog;->findViewById(I)Landroid/view/View;
    move-result-object v1
    check-cast v1, Landroid/widget/TextView;
    const-string v2, "danger_force_all"
    invoke-virtual {v2, p1}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v2
    if-eqz v2, :danger_regular_title
    const-string v2, "danger_force_all_title"
    invoke-direct {p0, v2}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v2
    invoke-virtual {v1, v2}, Landroid/widget/TextView;->setText(Ljava/lang/CharSequence;)V
    goto :danger_title_done
    :danger_regular_title
    invoke-virtual {v1, p1}, Landroid/widget/TextView;->setText(Ljava/lang/CharSequence;)V
    :danger_title_done

    iget-object v1, p0, Ldanger/DevExperimentsActivity;->b:Landroid/content/SharedPreferences;
    const-string v2, "__server__"
    invoke-interface {v1, p1, v2}, Landroid/content/SharedPreferences;->getString(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v4
    const-string v1, "danger_choice_current"
    const-string v2, "id"
    invoke-direct {p0, v1, v2}, Ldanger/DevExperimentsActivity;->r(Ljava/lang/String;Ljava/lang/String;)I
    move-result v1
    invoke-virtual {v0, v1}, Landroid/app/Dialog;->findViewById(I)Landroid/view/View;
    move-result-object v1
    check-cast v1, Landroid/widget/TextView;
    new-instance v2, Ljava/lang/StringBuilder;
    invoke-direct {v2}, Ljava/lang/StringBuilder;-><init>()V
    const-string v3, "danger_current_override"
    invoke-direct {p0, v3}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v3
    invoke-virtual {v2, v3}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    const-string v3, "__server__"
    invoke-virtual {v3, v4}, Ljava/lang/String;->equals(Ljava/lang/Object;)Z
    move-result v3
    if-eqz v3, :danger_current_raw
    const-string v3, "danger_from_server"
    invoke-direct {p0, v3}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v4
    :danger_current_raw
    invoke-virtual {v2, v4}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    invoke-virtual {v2}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v2
    invoke-virtual {v1, v2}, Landroid/widget/TextView;->setText(Ljava/lang/CharSequence;)V

    const-string v1, "danger_choice_server"
    const-string v2, "__server__"
    invoke-direct {p0, v0, v1, v2, p1}, Ldanger/DevExperimentsActivity;->bind(Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V
    const-string v1, "danger_choice_default"
    const-string v2, "default"
    invoke-direct {p0, v0, v1, v2, p1}, Ldanger/DevExperimentsActivity;->bind(Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V
    const-string v1, "danger_choice_on"
    const-string v2, "on"
    invoke-direct {p0, v0, v1, v2, p1}, Ldanger/DevExperimentsActivity;->bind(Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V
    const-string v1, "danger_choice_on1"
    const-string v2, "on1"
    invoke-direct {p0, v0, v1, v2, p1}, Ldanger/DevExperimentsActivity;->bind(Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V
    const-string v1, "danger_choice_control"
    const-string v2, "control"
    invoke-direct {p0, v0, v1, v2, p1}, Ldanger/DevExperimentsActivity;->bind(Landroid/app/Dialog;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V

    invoke-virtual {v0}, Landroid/app/Dialog;->show()V
    invoke-virtual {v0}, Landroid/app/Dialog;->getWindow()Landroid/view/Window;
    move-result-object v1
    new-instance v2, Landroid/graphics/drawable/ColorDrawable;
    const/4 v3, 0x0
    invoke-direct {v2, v3}, Landroid/graphics/drawable/ColorDrawable;-><init>(I)V
    invoke-virtual {v1, v2}, Landroid/view/Window;->setBackgroundDrawable(Landroid/graphics/drawable/Drawable;)V
    const/4 v2, -0x1
    const/4 v3, -0x2
    invoke-virtual {v1, v2, v3}, Landroid/view/Window;->setLayout(II)V
    const/16 v2, 0x11
    invoke-virtual {v1, v2}, Landroid/view/Window;->setGravity(I)V
    return-void
.end method

.method public applyChoice(Ljava/lang/String;Ljava/lang/String;Landroid/app/Dialog;)V
    .locals 3
    iget-object v0, p0, Ldanger/DevExperimentsActivity;->b:Landroid/content/SharedPreferences;
    invoke-interface {v0}, Landroid/content/SharedPreferences;->edit()Landroid/content/SharedPreferences$Editor;
    move-result-object v0
    invoke-interface {v0, p1, p2}, Landroid/content/SharedPreferences$Editor;->putString(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;
    move-result-object v0
    invoke-interface {v0}, Landroid/content/SharedPreferences$Editor;->apply()V
    invoke-virtual {p3}, Landroid/app/Dialog;->dismiss()V
    const-string v0, "danger_override_saved"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v0
    const/4 v1, 0x0
    invoke-static {p0, v0, v1}, Landroid/widget/Toast;->makeText(Landroid/content/Context;Ljava/lang/CharSequence;I)Landroid/widget/Toast;
    move-result-object v2
    invoke-virtual {v2}, Landroid/widget/Toast;->show()V
    invoke-direct {p0}, Ldanger/DevExperimentsActivity;->updateForce()V
    return-void
.end method

.method private resetDefaults()V
    .locals 4
    iget-object v0, p0, Ldanger/DevExperimentsActivity;->b:Landroid/content/SharedPreferences;
    invoke-interface {v0}, Landroid/content/SharedPreferences;->edit()Landroid/content/SharedPreferences$Editor;
    move-result-object v0
    invoke-interface {v0}, Landroid/content/SharedPreferences$Editor;->clear()Landroid/content/SharedPreferences$Editor;
    move-result-object v0
    const-string v1, "danger_force_all"
    const-string v2, "__server__"
    invoke-interface {v0, v1, v2}, Landroid/content/SharedPreferences$Editor;->putString(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;
    move-result-object v0
    invoke-interface {v0}, Landroid/content/SharedPreferences$Editor;->apply()V
    const-string v0, "danger_defaults_restored"
    invoke-direct {p0, v0}, Ldanger/DevExperimentsActivity;->s(Ljava/lang/String;)Ljava/lang/String;
    move-result-object v0
    const/4 v1, 0x0
    invoke-static {p0, v0, v1}, Landroid/widget/Toast;->makeText(Landroid/content/Context;Ljava/lang/CharSequence;I)Landroid/widget/Toast;
    move-result-object v3
    invoke-virtual {v3}, Landroid/widget/Toast;->show()V
    invoke-direct {p0}, Ldanger/DevExperimentsActivity;->updateForce()V
    return-void
.end method

.method public onTouch(Landroid/view/View;Landroid/view/MotionEvent;)Z
    .locals 5
    invoke-virtual {p2}, Landroid/view/MotionEvent;->getActionMasked()I
    move-result v0
    if-nez v0, :move_check
    invoke-virtual {p2}, Landroid/view/MotionEvent;->getRawY()F
    move-result v1
    iput v1, p0, Ldanger/DevExperimentsActivity;->d:F
    const/4 v0, 0x1
    return v0
    :move_check
    const/4 v1, 0x2
    if-ne v0, v1, :up_check
    invoke-virtual {p2}, Landroid/view/MotionEvent;->getRawY()F
    move-result v1
    iget v2, p0, Ldanger/DevExperimentsActivity;->d:F
    sub-float/2addr v1, v2
    const/4 v2, 0x0
    cmpg-float v3, v1, v2
    if-gez v3, :translate
    move v1, v2
    :translate
    iget-object v2, p0, Ldanger/DevExperimentsActivity;->c:Landroid/view/View;
    invoke-virtual {v2, v1}, Landroid/view/View;->setTranslationY(F)V
    const/4 v0, 0x1
    return v0
    :up_check
    const/4 v1, 0x1
    if-ne v0, v1, :return_true
    invoke-virtual {p2}, Landroid/view/MotionEvent;->getRawY()F
    move-result v2
    iget v3, p0, Ldanger/DevExperimentsActivity;->d:F
    sub-float/2addr v2, v3
    const/high16 v3, 0x43340000    # 180.0f
    cmpl-float v4, v2, v3
    if-lez v4, :snap_back
    invoke-virtual {p0}, Landroid/app/Activity;->finish()V
    return v1
    :snap_back
    iget-object v2, p0, Ldanger/DevExperimentsActivity;->c:Landroid/view/View;
    invoke-virtual {v2}, Landroid/view/View;->animate()Landroid/view/ViewPropertyAnimator;
    move-result-object v2
    const/4 v3, 0x0
    invoke-virtual {v2, v3}, Landroid/view/ViewPropertyAnimator;->translationY(F)Landroid/view/ViewPropertyAnimator;
    move-result-object v2
    const-wide/16 v3, 0xb4
    invoke-virtual {v2, v3, v4}, Landroid/view/ViewPropertyAnimator;->setDuration(J)Landroid/view/ViewPropertyAnimator;
    move-result-object v2
    invoke-virtual {v2}, Landroid/view/ViewPropertyAnimator;->start()V
    :return_true
    return v1
.end method

.method public afterTextChanged(Landroid/text/Editable;)V
    .locals 2
    iget-object v0, p0, Ldanger/DevExperimentsActivity;->a:Landroid/widget/ArrayAdapter;
    invoke-virtual {v0}, Landroid/widget/ArrayAdapter;->getFilter()Landroid/widget/Filter;
    move-result-object v0
    invoke-interface {p1}, Ljava/lang/CharSequence;->toString()Ljava/lang/String;
    move-result-object v1
    invoke-virtual {v0, v1}, Landroid/widget/Filter;->filter(Ljava/lang/CharSequence;)V
    return-void
.end method

.method public beforeTextChanged(Ljava/lang/CharSequence;III)V
    .locals 0
    return-void
.end method

.method public onTextChanged(Ljava/lang/CharSequence;III)V
    .locals 0
    return-void
.end method

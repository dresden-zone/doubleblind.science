import {Component, Input, ViewChild} from '@angular/core';
import { CommonModule } from '@angular/common';
import {ProjectService} from "../../core/data/project.service";
import {IconTrashComponent} from "../../core/icons/icon-trash/icon-trash.component";
import {ButtonComponent, TextFieldComponent, DropdownComponent, OptionComponent} from "@feel/form";
import {CardComponent} from "../../core/components/card/card.component";
import {IconEyeComponent} from "../../core/icons/icon-eye/icon-eye.component";
import {FormControl, FormGroup, ReactiveFormsModule, Validators} from "@angular/forms";
import {NotificationService} from "@feel/notification";

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule, IconTrashComponent, ButtonComponent, CardComponent, IconEyeComponent, TextFieldComponent, TextFieldComponent, ButtonComponent, DropdownComponent, ReactiveFormsModule, OptionComponent, ButtonComponent, ButtonComponent, ButtonComponent],
  templateUrl: './projects.component.html',
  styleUrl: './projects.component.scss'
})
export class ProjectsComponent {
  protected projects = this.projectService.getUserProjects();

  constructor(
    private readonly projectService:ProjectService,
    private readonly notificationService: NotificationService,
  ) {
    this.form.valueChanges.subscribe(console.log);
  }

  @Input()
  public projectName: string | null = null;

  @Input()
  public projectRepo: string | null = null;


  protected form = new FormGroup( {
    name: new FormControl(null, [Validators.required, Validators.minLength(6)]),
    repo: new FormControl(null, [Validators.required, Validators.pattern(/.+\/.+/)]),
  })

  protected visit_website(subdomain: string) {
      location.href=`https://${subdomain}.science.tanneberger.me`;
  }

  protected validate_input_and_submit() {
    if (!this.form.valid) {
      console.log("invalid form!");
      return;
    }

    const value = this.form.value;

    this.projectService.create(value.name!, value.repo!)
      .subscribe({
        next: () => {
          this.notificationService.success(`Successfully Created Project`);
        },
        error: err => {
          console.error(err);
          this.notificationService.error(`Failed to Create Project`);
        },
      });
  }
}

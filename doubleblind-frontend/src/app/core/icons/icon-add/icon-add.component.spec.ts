import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IconAddComponent } from './icon-add.component';

describe('IconAddComponent', () => {
  let component: IconAddComponent;
  let fixture: ComponentFixture<IconAddComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [IconAddComponent]
    });
    fixture = TestBed.createComponent(IconAddComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
